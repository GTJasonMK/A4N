use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ConnectionTrait, TransactionTrait};

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
