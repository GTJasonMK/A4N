use super::super::{
    generated_prompt_status,
    preservation::{prompt_sync_draft_fields_match, prompt_sync_generation_fields_changed},
};
use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{ConnectionTrait, Set};

use crate::video::{self, VideoDbError};
use a4n_domain::video_prompt::ChapterVideoPromptSyncDraft;

pub(in crate::video::prompts::consistency::sync) fn prompt_active_model(
    draft: &ChapterVideoPromptSyncDraft,
    now: chrono::NaiveDateTime,
) -> chapter_video_prompt::ActiveModel {
    let fields = &draft.fields;
    chapter_video_prompt::ActiveModel {
        project_id: Set(fields.project_id),
        chapter_id: Set(fields.chapter_id),
        style: Set(fields.style.clone()),
        prompt_type: Set(fields.prompt_type.clone()),
        workflow_mode: Set(fields.workflow_mode.clone()),
        shot_number: Set(fields.shot_number),
        title: Set(fields.title.clone()),
        scene_summary: Set(fields.scene_summary.clone()),
        visual_prompt: Set(fields.visual_prompt.clone()),
        motion_prompt: Set(fields.motion_prompt.clone()),
        negative_prompt: Set(fields.negative_prompt.clone()),
        duration_secs: Set(fields.duration_secs),
        aspect_ratio: Set(fields.aspect_ratio.clone()),
        reference_image_ids_json: Set(fields.reference_image_ids_json.clone()),
        status: Set(generated_prompt_status(&fields.prompt_type)),
        generated_video_id: Set(None),
        metadata_json: Set(fields.metadata_json.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
}

pub(in crate::video::prompts::consistency::sync) async fn replace_prompt_from_sync_draft<C>(
    db: &C,
    existing: chapter_video_prompt::Model,
    draft: ChapterVideoPromptSyncDraft,
    now: chrono::NaiveDateTime,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    if existing.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO
        && existing.status == chapter_video_prompt::STATUS_RUNNING
    {
        return Ok(existing);
    }
    if prompt_sync_draft_fields_match(&existing, &draft) {
        return Ok(existing);
    }
    let is_video_prompt = draft.fields.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO;
    let generation_fields_changed = prompt_sync_generation_fields_changed(&existing, &draft);
    if is_video_prompt && existing.generated_video_id.is_some() && generation_fields_changed {
        return Err(VideoDbError::InvalidArgument(
            "已存在由该视频提示词生成的视频，不能重新生成视频提示词；请先删除对应视频".to_string(),
        ));
    }
    let status = if is_video_prompt && generation_fields_changed {
        Some(chapter_video_prompt::STATUS_PENDING.to_string())
    } else if !is_video_prompt {
        Some(generated_prompt_status(&draft.fields.prompt_type))
    } else {
        None
    };
    video::replace_chapter_video_prompt_record(
        db,
        existing,
        draft.fields,
        status,
        is_video_prompt && generation_fields_changed,
        now,
    )
    .await
}
