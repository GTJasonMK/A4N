use super::super::{
    generated_prompt_status,
    preservation::{prompt_sync_draft_fields_match, prompt_sync_generation_fields_changed},
};
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;

use crate::video::{self, VideoDbError};
use crate::video_prompt::ChapterVideoPromptSyncDraft;

pub(in crate::video_prompt::sync) async fn insert_prompt_from_sync_draft<C>(
    db: &C,
    draft: ChapterVideoPromptSyncDraft,
    now: chrono::NaiveDateTime,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let status = generated_prompt_status(&draft.fields.prompt_type);
    video::insert_chapter_video_prompt_record(db, draft.fields, status, now).await
}

pub(in crate::video_prompt::sync) async fn replace_prompt_from_sync_draft<C>(
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
