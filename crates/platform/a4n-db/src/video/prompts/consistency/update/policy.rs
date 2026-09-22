use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use a4n_domain::source_text_hash;
use a4n_domain::video_prompt::{ChapterVideoPromptPatch, UpdateChapterVideoPromptRecordInput};
use a4n_video_contract::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2,
    update_video_segment_recipe_metadata, video_segment_content_fingerprint,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter,
    QuerySelect, TransactionTrait, sea_query::Expr,
};
use serde_json::Value;

use crate::video::{self, VideoDbError};

use super::{
    super::{ensure_chapter_video_prompt_stage_mutable, lock_chapter_video_prompt_scope},
    metadata::sync_chapter_video_prompt_metadata_after_update,
};

pub async fn update_chapter_video_prompt_with_policy<C>(
    db: &C,
    prompt_id: i64,
    input: ChapterVideoPromptPatch,
    allow_downstream: bool,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let row = update_chapter_video_prompt_with_policy_in(&txn, prompt_id, input, allow_downstream)
        .await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(row)
}

pub async fn update_chapter_video_segment_plan_metadata<C>(
    db: &C,
    prompt_id: i64,
    expected_updated_at: chrono::NaiveDateTime,
    metadata_json: Value,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let result = chapter_video_prompt::Entity::update_many()
        .col_expr(
            chapter_video_prompt::Column::MetadataJson,
            Expr::value(metadata_json),
        )
        .col_expr(
            chapter_video_prompt::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(chapter_video_prompt::Column::Id.eq(prompt_id))
        .filter(chapter_video_prompt::Column::UpdatedAt.eq(expected_updated_at))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(VideoDbError::InvalidArgument(
            "片段规划已被其他操作修改，请刷新后重试".to_string(),
        ));
    }
    chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频提示词不存在".to_string()))
}

pub(super) async fn update_chapter_video_prompt_with_policy_in<C>(
    db: &C,
    prompt_id: i64,
    input: ChapterVideoPromptPatch,
    allow_downstream: bool,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let ChapterVideoPromptPatch {
        visual_prompt,
        motion_prompt,
        negative_prompt,
        duration_secs,
        reference_image_ids_json,
    } = input;
    validate_prompt_duration_update(duration_secs)?;
    let prompt = video::chapter_video_prompt_record(db, prompt_id).await?;
    if prompt.status == chapter_video_prompt::STATUS_RUNNING {
        return Err(VideoDbError::InvalidState {
            current: prompt.status,
            operation: "编辑视频提示词".to_string(),
        });
    }
    video::ensure_video_chapter_belongs_to_project_record(db, prompt.project_id, prompt.chapter_id)
        .await?;
    if !allow_downstream {
        ensure_chapter_video_prompt_stage_mutable(db, &prompt, "编辑").await?;
    }
    if let Some(value) = reference_image_ids_json.as_ref() {
        video::validated_video_reference_image_records(
            db,
            prompt.project_id,
            prompt.chapter_id,
            value.as_ref(),
        )
        .await?;
    }
    let visual_prompt = visual_prompt
        .map(|value| required_prompt_text(&value, "视频提示词 visual_prompt"))
        .transpose()?;
    let motion_prompt = motion_prompt.map(normalize_optional_prompt_text);
    let negative_prompt = negative_prompt.map(normalize_optional_prompt_text);
    let was_video_prompt = prompt.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO;
    let content_changed = visual_prompt
        .as_ref()
        .is_some_and(|value| prompt.visual_prompt != *value)
        || motion_prompt
            .as_ref()
            .is_some_and(|value| prompt.motion_prompt != *value)
        || negative_prompt
            .as_ref()
            .is_some_and(|value| prompt.negative_prompt != *value)
        || duration_secs.is_some_and(|value| prompt.duration_secs != value)
        || reference_image_ids_json
            .as_ref()
            .is_some_and(|value| prompt.reference_image_ids_json != *value);
    let final_visual_prompt = visual_prompt
        .as_deref()
        .unwrap_or(prompt.visual_prompt.as_str())
        .to_string();
    let final_negative_prompt = negative_prompt
        .as_ref()
        .map(|value| value.as_deref())
        .unwrap_or(prompt.negative_prompt.as_deref())
        .map(ToString::to_string);
    let final_duration_secs = duration_secs.unwrap_or(prompt.duration_secs);
    let final_reference_image_ids_json = reference_image_ids_json
        .clone()
        .unwrap_or_else(|| prompt.reference_image_ids_json.clone());
    let metadata_json = sync_chapter_video_prompt_metadata_after_update(
        &prompt.prompt_type,
        prompt.metadata_json.clone(),
        &final_visual_prompt,
        final_negative_prompt.as_deref(),
        final_duration_secs,
        final_reference_image_ids_json.as_ref(),
    )?;
    video::update_chapter_video_prompt_record(
        db,
        prompt,
        UpdateChapterVideoPromptRecordInput {
            visual_prompt,
            motion_prompt,
            negative_prompt,
            duration_secs,
            reference_image_ids_json,
            metadata_json,
            reset_video_binding_to_pending: was_video_prompt
                && content_changed
                && !allow_downstream,
        },
    )
    .await
}

#[derive(Debug, FromQueryResult)]
struct SegmentPlanRecipeUpdateRow {
    id: i64,
    project_id: i64,
    chapter_id: i64,
    style: Option<String>,
    prompt_type: String,
    workflow_mode: String,
    status: String,
    metadata_json: Option<Value>,
}

#[derive(Debug, FromQueryResult)]
struct AdoptedVideoValidationRow {
    project_id: i64,
    chapter_id: Option<i64>,
    workflow_mode: Option<String>,
    file_path: Option<String>,
    status: String,
    metadata_json: Option<Value>,
}

pub async fn update_chapter_video_segment_recipe<C>(
    db: &C,
    prompt_id: i64,
    recipe: VideoSegmentExecutionRecipeV1,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let recipes = update_chapter_video_segment_recipe_in(&txn, prompt_id, recipe).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(recipes)
}

async fn update_chapter_video_segment_recipe_in<C>(
    db: &C,
    prompt_id: i64,
    recipe: VideoSegmentExecutionRecipeV1,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, VideoDbError>
where
    C: ConnectionTrait,
{
    let (candidate_project_id, candidate_chapter_id) =
        video::chapter_video_prompt_scope_by_id(db, prompt_id)
            .await?
            .ok_or_else(|| VideoDbError::NotFound("视频提示词不存在".to_string()))?;
    let chapter = lock_chapter_video_prompt_scope(db, candidate_chapter_id).await?;
    let query = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::ProjectId)
        .column(chapter_video_prompt::Column::ChapterId)
        .column(chapter_video_prompt::Column::Style)
        .column(chapter_video_prompt::Column::PromptType)
        .column(chapter_video_prompt::Column::WorkflowMode)
        .column(chapter_video_prompt::Column::Status)
        .column(chapter_video_prompt::Column::MetadataJson);
    let query = if db.get_database_backend() == DbBackend::Sqlite {
        query
    } else {
        query.lock_exclusive()
    };
    let plan = query
        .into_model::<SegmentPlanRecipeUpdateRow>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频提示词不存在".to_string()))?;
    if plan.project_id != candidate_project_id || plan.chapter_id != candidate_chapter_id {
        return Err(VideoDbError::InvalidArgument(
            "视频提示词归属在更新期间发生变化，请重试".to_string(),
        ));
    }
    if plan.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        || plan.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        || plan.status != chapter_video_prompt::STATUS_COMPLETED
    {
        return Err(VideoDbError::InvalidArgument(
            "片段执行方案只能写入当前已完成的共享片段规划".to_string(),
        ));
    }
    video::ensure_video_chapter_belongs_to_project_record(db, plan.project_id, plan.chapter_id)
        .await?;
    let current_prompt_id = video::completed_shared_segment_plan_id_for_source_hash(
        db,
        plan.project_id,
        plan.chapter_id,
        &source_text_hash(&chapter),
    )
    .await?;
    if current_prompt_id != Some(plan.id) {
        return Err(VideoDbError::InvalidArgument(
            "目标片段规划不是当前规划，请刷新后重试".to_string(),
        ));
    }
    let plan_style = plan
        .style
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| VideoDbError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
    let requested_adopted_video = recipe
        .adopted_video_id
        .map(|video_id| (video_id, recipe.clone()));
    let (metadata, recipes) =
        update_video_segment_recipe_metadata(plan.metadata_json.as_ref(), plan_style, recipe)
            .map_err(VideoDbError::InvalidArgument)?;
    if let Some((video_id, recipe)) = requested_adopted_video {
        validate_adopted_video(db, &plan, &recipe, video_id).await?;
    }
    let result = chapter_video_prompt::Entity::update_many()
        .col_expr(
            chapter_video_prompt::Column::MetadataJson,
            Expr::value(metadata),
        )
        .col_expr(
            chapter_video_prompt::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(chapter_video_prompt::Column::Id.eq(plan.id))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(VideoDbError::NotFound("视频提示词不存在".to_string()));
    }
    Ok(recipes)
}

async fn validate_adopted_video<C>(
    db: &C,
    plan: &SegmentPlanRecipeUpdateRow,
    recipe: &VideoSegmentExecutionRecipeV1,
    video_id: i64,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let video = generated_video::Entity::find_by_id(video_id)
        .select_only()
        .column(generated_video::Column::ProjectId)
        .column(generated_video::Column::ChapterId)
        .column(generated_video::Column::WorkflowMode)
        .column(generated_video::Column::FilePath)
        .column(generated_video::Column::Status)
        .column(generated_video::Column::MetadataJson)
        .into_model::<AdoptedVideoValidationRow>()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("视频 #{video_id} 不存在")))?;
    let valid = video.project_id == plan.project_id
        && video.chapter_id == Some(plan.chapter_id)
        && video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
        && video
            .file_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|path| !path.is_empty())
        && video.workflow_mode.as_deref() == Some(recipe.workflow_mode.as_str())
        && adopted_video_identity_matches(
            plan.id,
            plan.metadata_json.as_ref(),
            video.metadata_json.as_ref(),
            recipe,
        )?;
    if !valid {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 #{video_id} 不属于当前规划片段 {} 的所选执行方案",
            recipe.segment_number
        )));
    }
    Ok(())
}

fn adopted_video_identity_matches(
    plan_id: i64,
    plan_metadata: Option<&Value>,
    video_metadata: Option<&Value>,
    recipe: &VideoSegmentExecutionRecipeV1,
) -> Result<bool, VideoDbError> {
    let Some(plan_metadata) = plan_metadata else {
        return Ok(false);
    };
    let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(
        plan_metadata.get("segments").cloned().ok_or_else(|| {
            VideoDbError::InvalidArgument("当前片段规划缺少 segments".to_string())
        })?,
    )
    .map_err(|error| VideoDbError::InvalidArgument(format!("当前片段规划无效：{error}")))?;
    let segment = segments
        .iter()
        .find(|segment| segment.segment_number == recipe.segment_number)
        .ok_or_else(|| {
            VideoDbError::InvalidArgument(format!("当前片段规划缺少片段 {}", recipe.segment_number))
        })?;
    let Some(plan_identity) = plan_metadata
        .get("video_execution_identity")
        .and_then(Value::as_object)
    else {
        return Ok(false);
    };
    let Some(video_identity) = video_metadata
        .and_then(|metadata| metadata.get("video_execution_identity"))
        .and_then(Value::as_object)
    else {
        return Ok(false);
    };
    let segment_fingerprint =
        video_segment_content_fingerprint(segment).map_err(VideoDbError::InvalidArgument)?;
    let recipe_fingerprint = recipe
        .generation_fingerprint()
        .map_err(VideoDbError::InvalidArgument)?;
    let contract_epoch = Some(u64::from(VIDEO_EXECUTION_CONTRACT_EPOCH));
    Ok(
        plan_identity.get("contract_epoch").and_then(Value::as_u64) == contract_epoch
            && video_identity.get("contract_epoch").and_then(Value::as_u64) == contract_epoch
            && ["source_content_sha256", "style_sha256", "aspect_ratio"]
                .into_iter()
                .all(|field| {
                    plan_identity
                        .get(field)
                        .is_some_and(|expected| video_identity.get(field) == Some(expected))
                })
            && video_identity
                .get("segment_plan_prompt_id")
                .and_then(Value::as_i64)
                == Some(plan_id)
            && video_identity.get("source_shot_id").and_then(Value::as_str)
                == Some(segment.source_shot_id.as_str())
            && video_identity
                .get("segment_index_in_shot")
                .and_then(Value::as_i64)
                == Some(i64::from(segment.segment_index_in_shot))
            && video_identity.get("segment_sha256").and_then(Value::as_str)
                == Some(segment_fingerprint.as_str())
            && video_identity
                .get("segment_recipe_fingerprint")
                .and_then(Value::as_str)
                == Some(recipe_fingerprint.as_str()),
    )
}

fn required_prompt_text(value: &str, field: &str) -> Result<String, VideoDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(VideoDbError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn normalize_optional_prompt_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn validate_prompt_duration_update(duration_secs: Option<i32>) -> Result<(), VideoDbError> {
    if duration_secs.is_some_and(|value| value <= 0) {
        return Err(VideoDbError::InvalidArgument(
            "视频提示词 duration_secs 必须是正整数".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_video_contract::default_video_segment_recipes;
    use serde_json::json;

    #[test]
    fn prompt_duration_update_requires_positive_integer() {
        assert!(validate_prompt_duration_update(None).is_ok());
        assert!(validate_prompt_duration_update(Some(1)).is_ok());
        assert!(matches!(
            validate_prompt_duration_update(Some(0)),
            Err(VideoDbError::InvalidArgument(message))
                if message == "视频提示词 duration_secs 必须是正整数"
        ));
    }

    #[test]
    fn adopted_video_identity_uses_stable_segment_target_after_renumbering() {
        let segment = serde_json::from_value::<VideoSegmentPlanV2>(json!({
            "schema_version": "video_segment_plan_v2",
            "segment_number": 2,
            "source_shot_id": "shot-1",
            "segment_index_in_shot": 1,
            "shot_segment_count": 1,
            "covered_beat_ids": [],
            "title": "片段",
            "scene_summary": "场景",
            "blocking_summary": "调度",
            "spatial_anchors": [],
            "emotion_action_goals": [],
            "video_intent": {
                "hook": "h",
                "audience_effect": "a",
                "rhythm": "r",
                "visual_priority": "v"
            },
            "emotional_arc": {"start": "s", "turn": "t", "end": "e", "function": "f"},
            "performance_plan": {
                "movement_path": "m",
                "body_performance": "b",
                "gaze_and_hands": "g",
                "pause_or_reaction": "p",
                "camera_relationship": "c",
                "time_blocks": [],
                "action_script": []
            },
            "material_coverage": {
                "required_entities": [],
                "missing_or_weak_references": [],
                "risk_note": "r"
            },
            "transition_to_next": null,
            "source_span_ids": ["span-1"],
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "reference_image_ids": []
        }))
        .unwrap();
        let recipe =
            default_video_segment_recipes(std::slice::from_ref(&segment), "cinematic", "zh_cn")
                .unwrap()
                .remove(0);
        let segment_sha256 = video_segment_content_fingerprint(&segment).unwrap();
        let recipe_fingerprint = recipe.generation_fingerprint().unwrap();
        let plan_metadata = json!({
            "segments": [segment],
            "video_execution_identity": {
                "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
                "source_content_sha256": "source",
                "style_sha256": "style",
                "duration_secs": 6,
                "aspect_ratio": "16:9"
            }
        });
        let video_metadata = json!({
            "shot_number": 1,
            "video_execution_identity": {
                "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
                "source_content_sha256": "source",
                "style_sha256": "style",
                "duration_secs": 10,
                "aspect_ratio": "16:9",
                "segment_plan_prompt_id": 42,
                "segment_number": 1,
                "source_shot_id": "shot-1",
                "segment_index_in_shot": 1,
                "segment_sha256": segment_sha256,
                "segment_recipe_fingerprint": recipe_fingerprint
            }
        });

        assert!(
            adopted_video_identity_matches(
                42,
                Some(&plan_metadata),
                Some(&video_metadata),
                &recipe,
            )
            .unwrap()
        );
    }
}
