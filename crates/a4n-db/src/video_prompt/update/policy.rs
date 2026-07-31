use a4n_core::{
    models::{ai::generated_video, chapter::chapter_video_prompt},
    video_execution::{
        VIDEO_SEGMENT_RECIPES_METADATA_KEY, VideoSegmentExecutionRecipeV1,
        metadata_with_video_segment_recipe, propagate_adopted_video_change,
        video_segment_recipes_for_plan_from_metadata,
    },
};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect, TransactionTrait};
use serde_json::Value;

use crate::video::{self, VideoDbError};

use super::{
    super::{UpdateChapterVideoPromptInput, ensure_chapter_video_prompt_stage_mutable},
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
        segment_recipe,
    } = input;
    if segment_recipe.is_some() && metadata_patch.is_some() {
        return Err(VideoDbError::InvalidArgument(
            "片段执行方案不能与通用 metadata_patch 同时更新".to_string(),
        ));
    }
    let prompt = prompt_for_update(db, prompt_id, segment_recipe.is_some()).await?;
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
        || metadata_patch.is_some()
        || segment_recipe.is_some();
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
    if let Some(serde_json::Value::Object(patch)) = metadata_patch {
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
    if let Some(recipe) = segment_recipe {
        metadata_json =
            Some(merge_segment_recipe(db, &prompt, metadata_json.as_ref(), recipe).await?);
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

async fn prompt_for_update<C>(
    db: &C,
    prompt_id: i64,
    lock_project: bool,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let candidate = video::chapter_video_prompt_record(db, prompt_id).await?;
    if !lock_project || db.get_database_backend() == DbBackend::Sqlite {
        return Ok(candidate);
    }
    crate::project::project_by_id_for_update(db, candidate.project_id)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频提示词所属项目不存在".to_string()))?;
    let locked = chapter_video_prompt::Entity::find_by_id(prompt_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound("视频提示词不存在".to_string()))?;
    if locked.project_id != candidate.project_id {
        return Err(VideoDbError::InvalidArgument(
            "视频提示词项目归属在更新期间发生变化，请重试".to_string(),
        ));
    }
    Ok(locked)
}

async fn merge_segment_recipe<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    metadata: Option<&Value>,
    mut recipe: VideoSegmentExecutionRecipeV1,
) -> Result<Value, VideoDbError>
where
    C: ConnectionTrait,
{
    if prompt.prompt_type != "frame_segment_plan"
        || prompt.workflow_mode != "shared_segment_plan"
        || prompt.status != "completed"
    {
        return Err(VideoDbError::InvalidArgument(
            "片段执行方案只能写入当前已完成的共享片段规划".to_string(),
        ));
    }
    let plan_style = prompt
        .style
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| VideoDbError::InvalidArgument("当前片段规划缺少 style".to_string()))?;
    let recipes = video_segment_recipes_for_plan_from_metadata(metadata, plan_style)
        .map_err(VideoDbError::InvalidArgument)?;
    let index = recipes
        .iter()
        .position(|row| row.segment_number == recipe.segment_number)
        .ok_or_else(|| {
            VideoDbError::InvalidArgument(format!(
                "当前片段规划缺少片段 {} 的执行方案",
                recipe.segment_number
            ))
        })?;
    let existing = recipes[index].clone();

    if let Some(source_video_id) = recipe.continuity_source_video_id
        && !recipes.iter().any(|candidate| {
            candidate.segment_number < recipe.segment_number
                && candidate.adopted_video_id == Some(source_video_id)
        })
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "续接来源视频 #{source_video_id} 必须是当前规划更早片段已采用的视频"
        )));
    }

    let requested_adopted_video_id = recipe.adopted_video_id;
    if recipe.adopted_video_id.is_none()
        && recipe
            .generation_fingerprint()
            .map_err(VideoDbError::InvalidArgument)?
            == existing
                .generation_fingerprint()
                .map_err(VideoDbError::InvalidArgument)?
    {
        recipe.adopted_video_id = existing.adopted_video_id;
    }
    if let Some(video_id) = requested_adopted_video_id {
        validate_adopted_video(db, prompt, &recipe, video_id).await?;
    }

    let previous_adopted_video_id = existing.adopted_video_id;
    let next_adopted_video_id = recipe.adopted_video_id;
    let target_segment_number = recipe.segment_number;
    let mut metadata = metadata_with_video_segment_recipe(metadata, plan_style, recipe)
        .map_err(VideoDbError::InvalidArgument)?;
    let mut recipes = video_segment_recipes_for_plan_from_metadata(Some(&metadata), plan_style)
        .map_err(VideoDbError::InvalidArgument)?;
    propagate_adopted_video_change(
        &mut recipes,
        target_segment_number,
        previous_adopted_video_id,
        next_adopted_video_id,
    );

    let object = metadata.as_object_mut().ok_or_else(|| {
        VideoDbError::InvalidArgument("片段规划 metadata_json 必须为 JSON 对象".to_string())
    })?;
    object.insert(
        VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
        serde_json::to_value(recipes).map_err(|error| {
            VideoDbError::InvalidArgument(format!("序列化片段执行方案失败：{error}"))
        })?,
    );
    Ok(metadata)
}

async fn validate_adopted_video<C>(
    db: &C,
    plan: &chapter_video_prompt::Model,
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
