use a4n_core::{dependency_conflict::DownstreamConflictStrategy, error::AppError};
use a4n_orchestrator::video::{VideoPromptApplicationContext, VideoPromptDownstreamCleanupPort};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::{
    AppState,
    actions::{image_records, video_records},
    prompt_renderer::RuntimePromptRenderer,
};

pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
pub use a4n_orchestrator::video::{
    ChapterVideoPromptSummary, DeleteChapterVideoPromptGroupResult,
    GenerateChapterVideoPromptInput, GeneratePromptOptionsInput, OptimizeChapterVideoPromptInput,
    VideoPromptTaskScope, VideoPromptUpdateApplicationInput as UpdateChapterVideoPromptInput,
};
pub use a4n_orchestrator::video::{
    VideoPromptChunk, VideoPromptProgress, VideoPromptRequestPreview,
};

mod preview;

use preview::{
    replan_video_prompt_request_preview_for_state, video_prompt_preview_to_model_request_preview,
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
            image_records::delete_image(self.state, self.user_id, *image_id)
                .await
                .map_err(internal_error)?;
        }
        Ok(())
    }
}

pub async fn chapter_video_prompt_task_scope(
    state: &AppState,
    input: &GenerateChapterVideoPromptInput,
) -> Result<VideoPromptTaskScope, String> {
    state
        .video()
        .video_prompt_task_scope_validated(input)
        .await
        .map_err(error_message)
}

pub async fn generate_chapter_video_frame_prompts(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let runtime_video_config = crate::runtime_config::current_video_model();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .video()
        .generate_chapter_video_frame_prompts_validated(
            VideoPromptApplicationContext {
                runtime_video_config: runtime_video_config.as_ref(),
                prompt_renderer: &prompt_renderer,
            },
            input,
            progress,
            chunks,
            cancel_token,
        )
        .await
        .map_err(error_message)
}

pub async fn generate_chapter_video_segment_plan(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let runtime_video_config = crate::runtime_config::current_video_model();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .video()
        .generate_chapter_video_segment_plan_validated(
            VideoPromptApplicationContext {
                runtime_video_config: runtime_video_config.as_ref(),
                prompt_renderer: &prompt_renderer,
            },
            input,
            progress,
            chunks,
            cancel_token,
        )
        .await
        .map_err(error_message)
}

pub async fn generate_chapter_video_prompts(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let runtime_video_config = crate::runtime_config::current_video_model();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .video()
        .generate_chapter_video_prompts_validated(
            VideoPromptApplicationContext {
                runtime_video_config: runtime_video_config.as_ref(),
                prompt_renderer: &prompt_renderer,
            },
            input,
            progress,
            chunks,
            cancel_token,
        )
        .await
        .map_err(error_message)
}

pub async fn preview_chapter_video_frame_prompt_requests(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<VideoPromptRequestPreview, String> {
    let runtime_video_config = crate::runtime_config::current_video_model();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let preview = state
        .video()
        .preview_chapter_video_frame_prompt_requests_validated(
            VideoPromptApplicationContext {
                runtime_video_config: runtime_video_config.as_ref(),
                prompt_renderer: &prompt_renderer,
            },
            input,
        )
        .await
        .map_err(error_message)?;
    replan_video_prompt_request_preview_for_state(state, preview).await
}

pub async fn preview_chapter_video_frame_prompt_model_request(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<ModelRequestPreview, String> {
    let preview = preview_chapter_video_frame_prompt_requests(state, input).await?;
    video_prompt_preview_to_model_request_preview(state, preview).await
}

pub async fn preview_chapter_video_prompt_requests(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<VideoPromptRequestPreview, String> {
    let runtime_video_config = crate::runtime_config::current_video_model();
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let preview = state
        .video()
        .preview_chapter_video_prompt_requests_validated(
            VideoPromptApplicationContext {
                runtime_video_config: runtime_video_config.as_ref(),
                prompt_renderer: &prompt_renderer,
            },
            input,
        )
        .await
        .map_err(error_message)?;
    replan_video_prompt_request_preview_for_state(state, preview).await
}

pub async fn preview_chapter_video_prompt_model_request(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
) -> Result<ModelRequestPreview, String> {
    let preview = preview_chapter_video_prompt_requests(state, input).await?;
    video_prompt_preview_to_model_request_preview(state, preview).await
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

pub async fn delete_chapter_video_prompt_group(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    group: &str,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<DeleteChapterVideoPromptGroupResult, String> {
    state
        .video()
        .delete_chapter_video_prompt_group_validated(
            user_id,
            project_id,
            chapter_id,
            group,
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
