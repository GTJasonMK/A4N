use a4n_core::{
    models::{ai::generated_video, chapter::chapter_video_prompt},
    source_spans::source_text_hash,
    video_execution::{VideoSegmentExecutionRecipeV1, update_video_segment_recipe_metadata},
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter,
    QuerySelect, TransactionTrait, sea_query::Expr,
};
use serde_json::Value;

use crate::video::{self, VideoDbError};

use super::{
    super::{
        UpdateChapterVideoPromptInput, ensure_chapter_video_prompt_stage_mutable,
        lock_chapter_video_prompt_scope,
    },
    metadata::sync_chapter_video_prompt_metadata_after_update,
};

pub async fn update_chapter_video_prompt_with_policy<C>(
    db: &C,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
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

pub(super) async fn update_chapter_video_prompt_with_policy_in<C>(
    db: &C,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
    allow_downstream: bool,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let UpdateChapterVideoPromptInput {
        visual_prompt,
        motion_prompt,
        negative_prompt,
        duration_secs,
        reference_image_ids_json,
        metadata_patch,
    } = input;
    let prompt = video::chapter_video_prompt_record(db, prompt_id).await?;
    if prompt.status == "running" {
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
    let was_video_prompt = prompt.prompt_type == "video_prompt";
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
            .is_some_and(|value| prompt.reference_image_ids_json != *value)
        || metadata_patch.is_some();
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
    let mut metadata_json = sync_chapter_video_prompt_metadata_after_update(
        &prompt.prompt_type,
        prompt.metadata_json.clone(),
        &final_visual_prompt,
        final_negative_prompt.as_deref(),
        final_duration_secs,
        final_reference_image_ids_json.as_ref(),
    )?;
    if let Some(metadata_patch) = metadata_patch {
        let serde_json::Value::Object(patch) = metadata_patch else {
            return Err(VideoDbError::InvalidArgument(
                "视频提示词 metadata_patch 必须是 JSON 对象".to_string(),
            ));
        };
        let root = metadata_json
            .get_or_insert_with(|| serde_json::Value::Object(Default::default()))
            .as_object_mut()
            .ok_or_else(|| {
                VideoDbError::InvalidArgument(
                    "视频提示词 metadata_json 必须是对象，不能保存片段执行设置".to_string(),
                )
            })?;
        for (key, value) in patch {
            if value.is_null() {
                root.remove(&key);
            } else {
                root.insert(key, value);
            }
        }
    }
    video::update_chapter_video_prompt_record(
        db,
        prompt,
        video::UpdateChapterVideoPromptRecordInput {
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
    if plan.prompt_type != "frame_segment_plan"
        || plan.workflow_mode != "shared_segment_plan"
        || plan.status != "completed"
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
        &source_text_hash(&chapter.content),
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
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(format!("视频 #{video_id} 不存在")))?;
    let identity = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("video_execution_identity"))
        .and_then(Value::as_object);
    let fingerprint = recipe
        .generation_fingerprint()
        .map_err(VideoDbError::InvalidArgument)?;
    let valid = video.project_id == plan.project_id
        && video.chapter_id == Some(plan.chapter_id)
        && video.status == "completed"
        && video
            .file_path
            .as_deref()
            .map(str::trim)
            .is_some_and(|path| !path.is_empty())
        && video.workflow_mode.as_deref() == Some(recipe.workflow_mode.as_str())
        && video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("shot_number"))
            .and_then(Value::as_i64)
            == Some(i64::from(recipe.segment_number))
        && identity
            .and_then(|value| value.get("segment_plan_prompt_id"))
            .and_then(Value::as_i64)
            == Some(plan.id)
        && identity
            .and_then(|value| value.get("segment_number"))
            .and_then(Value::as_i64)
            == Some(i64::from(recipe.segment_number))
        && identity
            .and_then(|value| value.get("segment_recipe_fingerprint"))
            .and_then(Value::as_str)
            == Some(fingerprint.as_str());
    if !valid {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频 #{video_id} 不属于当前规划片段 {} 的所选执行方案",
            recipe.segment_number
        )));
    }
    Ok(())
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
