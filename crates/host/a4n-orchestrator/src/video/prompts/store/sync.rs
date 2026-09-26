use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::video_prompt::{
    ChapterVideoPromptRecordFields, ChapterVideoPromptSyncDraft, ChapterVideoPromptSyncPolicy,
};
use a4n_error::AppError;

use super::contracts::NewChapterVideoPrompt;
use super::trace::prepare_generation_trace;
use crate::OrchestratorContext;
use crate::video::video_db_error;

impl OrchestratorContext {
    pub(in crate::video) async fn sync_chapter_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.write_chapter_prompts(chapter_id, drafts, ChapterVideoPromptSyncPolicy::default())
            .await
    }

    pub(in crate::video) async fn sync_chapter_prompts_preserving_generated_video_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.write_chapter_prompts(
            chapter_id,
            drafts,
            ChapterVideoPromptSyncPolicy {
                preserve_generated_video_prompts: true,
                ..Default::default()
            },
        )
        .await
    }

    pub(in crate::video) async fn sync_chapter_prompts_preserving_generated_frame_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        self.write_chapter_prompts(
            chapter_id,
            drafts,
            ChapterVideoPromptSyncPolicy {
                preserve_frame_prompts_with_generated_images: true,
                ..Default::default()
            },
        )
        .await
    }

    async fn write_chapter_prompts(
        &self,
        chapter_id: i64,
        drafts: Vec<NewChapterVideoPrompt>,
        policy: ChapterVideoPromptSyncPolicy,
    ) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
        if drafts.is_empty() {
            return Ok(Vec::new());
        }
        let drafts = drafts
            .into_iter()
            .map(prepare_prompt_sync_draft)
            .collect::<Result<Vec<_>, _>>()?;
        a4n_db::video::prompts::consistency::sync_chapter_video_prompts_with_generation_runs(
            &self.database,
            chapter_id,
            drafts,
            policy,
        )
        .await
        .map_err(video_db_error)
    }
}

fn prepare_prompt_sync_draft(
    draft: NewChapterVideoPrompt,
) -> Result<ChapterVideoPromptSyncDraft, AppError> {
    let generation_trace = draft
        .generation_trace
        .map(prepare_generation_trace)
        .transpose()?;
    let workflow_mode = draft.workflow_mode.as_str().to_string();
    Ok(ChapterVideoPromptSyncDraft {
        fields: ChapterVideoPromptRecordFields {
            project_id: draft.project_id,
            chapter_id: draft.chapter_id,
            style: draft.style,
            prompt_type: draft.prompt_type,
            workflow_mode,
            shot_number: draft.shot_number,
            title: draft.title,
            scene_summary: draft.scene_summary,
            visual_prompt: draft.visual_prompt,
            motion_prompt: draft.motion_prompt,
            negative_prompt: draft.negative_prompt,
            duration_secs: draft.duration_secs,
            aspect_ratio: draft.aspect_ratio,
            reference_image_ids_json: draft.reference_image_ids_json,
            metadata_json: draft.metadata_json,
        },
        generation_trace,
    })
}
