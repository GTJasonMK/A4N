use a4n_core::{dependency_conflict::DownstreamConflictStrategy, error::AppError};
use a4n_orchestrator::video::VideoPromptDownstreamCleanupPort;
use async_trait::async_trait;

use crate::{
    AppState,
    actions::{image_records, video_records},
    prompt_renderer::RuntimePromptRenderer,
};

pub use a4n_orchestrator::video::{
    ChapterVideoPromptSummary, LoadVideoSegmentPlanDraftInput, OptimizeChapterVideoPromptInput,
    UpdateVideoSegmentPlanDraftRecipeInput,
    VideoPromptUpdateApplicationInput as UpdateChapterVideoPromptInput,
};
pub use a4n_orchestrator::workflow::WorkflowRunDetailSummary;

struct RuntimeVideoPromptCleanup<'a> {
    state: &'a AppState,
    user_id: i64,
}

#[async_trait]
impl VideoPromptDownstreamCleanupPort for RuntimeVideoPromptCleanup<'_> {
    async fn delete_downstream_media(
        &self,
        image_ids: &[i64],
        video_ids: &[i64],
    ) -> Result<(), AppError> {
        for video_id in video_ids {
            video_records::delete_video(self.state, self.user_id, *video_id)
                .await
                .map_err(internal_error)?;
        }
        for image_id in image_ids {
            image_records::delete_image(self.state, self.user_id, *image_id)
                .await
                .map_err(internal_error)?;
        }
        Ok(())
    }
}

pub async fn list_chapter_video_prompts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    state
        .video()
        .list_chapter_video_prompts_validated(user_id, project_id, chapter_id)
        .await
        .map_err(error_message)
}

pub async fn load_video_segment_plan_draft(
    state: &AppState,
    user_id: i64,
    input: LoadVideoSegmentPlanDraftInput,
) -> Result<WorkflowRunDetailSummary, String> {
    state
        .video()
        .load_video_segment_plan_draft_validated(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn update_video_segment_plan_draft_recipe(
    state: &AppState,
    user_id: i64,
    input: UpdateVideoSegmentPlanDraftRecipeInput,
) -> Result<WorkflowRunDetailSummary, String> {
    state
        .video()
        .update_video_segment_plan_draft_recipe_validated(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn update_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: UpdateChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    state
        .video()
        .update_chapter_video_prompt_validated(
            user_id,
            prompt_id,
            input,
            &RuntimeVideoPromptCleanup { state, user_id },
        )
        .await
        .map_err(error_message)
}

pub async fn delete_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    state
        .video()
        .delete_chapter_video_prompt_validated(
            user_id,
            prompt_id,
            downstream_strategy,
            &RuntimeVideoPromptCleanup { state, user_id },
        )
        .await
        .map_err(error_message)
}

pub async fn optimize_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: OptimizeChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .video()
        .optimize_chapter_video_prompt_validated(
            user_id,
            prompt_id,
            input,
            &prompt_renderer,
            &RuntimeVideoPromptCleanup { state, user_id },
        )
        .await
        .map_err(error_message)
}

pub(crate) async fn resolve_project_chapter_video_source(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<a4n_orchestrator::video::ProjectChapterVideoSource, String> {
    state
        .video()
        .project_chapter_video_source(project_id, chapter_id, operation, writable)
        .await
        .map_err(error_message)
}

fn error_message(error: AppError) -> String {
    error.to_string()
}

fn internal_error(error: String) -> AppError {
    AppError::Internal(anyhow::anyhow!(error))
}
