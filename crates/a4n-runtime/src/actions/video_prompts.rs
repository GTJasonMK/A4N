use a4n_core::error::AppError;
use a4n_domain::DownstreamConflictStrategy;
use a4n_orchestrator::video::VideoPromptDownstreamCleanupPort;
use async_trait::async_trait;

use crate::{
    AppState,
    actions::{image_records, video_records},
};

pub use a4n_api_contract::video::{
    CommitInsertedSegmentDraftInput, DeleteChapterVideoPromptInput, DeleteInsertedSegmentInput,
    GetVideoTimelineInput, MoveInsertedSegmentInput, VideoTimelineSummary,
};
pub use a4n_core::video_execution::VideoSegmentExecutionRecipeV1;
pub use a4n_orchestrator::video::{
    ChapterVideoPromptSummary, LoadVideoSegmentPlanDraftInput, OptimizeChapterVideoPromptInput,
    UpdateChapterVideoPromptInput, UpdateVideoSegmentPlanDraftRecipeInput,
    VideoSegmentPlanDraftSummary,
};

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
            image_records::delete_image(
                self.state,
                self.user_id,
                image_records::DeleteImageInput {
                    image_id: *image_id,
                },
            )
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
        .orchestrator()
        .list_chapter_video_prompts_validated(user_id, project_id, chapter_id)
        .await
        .map_err(error_message)
}

pub async fn get_chapter_video_timeline(
    state: &AppState,
    user_id: i64,
    input: GetVideoTimelineInput,
) -> Result<VideoTimelineSummary, String> {
    state
        .orchestrator()
        .get_video_timeline(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn commit_inserted_segment_draft(
    state: &AppState,
    user_id: i64,
    input: CommitInsertedSegmentDraftInput,
) -> Result<VideoTimelineSummary, String> {
    state
        .orchestrator()
        .commit_inserted_segment_draft(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn move_inserted_segment(
    state: &AppState,
    user_id: i64,
    input: MoveInsertedSegmentInput,
) -> Result<VideoTimelineSummary, String> {
    state
        .orchestrator()
        .move_video_timeline_item(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn delete_inserted_segment(
    state: &AppState,
    user_id: i64,
    input: DeleteInsertedSegmentInput,
) -> Result<VideoTimelineSummary, String> {
    state
        .orchestrator()
        .delete_video_timeline_item(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn load_video_segment_plan_draft(
    state: &AppState,
    user_id: i64,
    input: LoadVideoSegmentPlanDraftInput,
) -> Result<VideoSegmentPlanDraftSummary, String> {
    state
        .orchestrator()
        .load_video_segment_plan_draft_validated(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn update_video_segment_plan_draft_recipe(
    state: &AppState,
    user_id: i64,
    input: UpdateVideoSegmentPlanDraftRecipeInput,
) -> Result<(), String> {
    state
        .orchestrator()
        .update_video_segment_plan_draft_recipe_validated(user_id, input)
        .await
        .map_err(error_message)
}

pub async fn update_chapter_video_segment_recipe(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    recipe: VideoSegmentExecutionRecipeV1,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, String> {
    state
        .orchestrator()
        .update_chapter_video_segment_recipe_validated(user_id, prompt_id, recipe)
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
        .orchestrator()
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
        .orchestrator()
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
    let prompt_renderer = state;
    state
        .orchestrator()
        .optimize_chapter_video_prompt_validated(
            user_id,
            prompt_id,
            input,
            prompt_renderer,
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
        .orchestrator()
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
