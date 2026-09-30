mod execution;
mod preview;

use crate::AppState;
use a4n_api_contract::generation_tasks::{GenerationTaskHandle, GenerationTaskStartError};
use a4n_api_contract::manga::MangaPipelinePageImageGenerator;
use a4n_error::AppError;
use a4n_image_host_contract::ImageGenerationProgress;

struct RuntimeMangaPipelinePageImageGenerator<'a> {
    state: &'a AppState,
}

#[async_trait::async_trait]
impl MangaPipelinePageImageGenerator for RuntimeMangaPipelinePageImageGenerator<'_> {
    async fn generate_page_image<'a>(
        &self,
        user_id: i64,
        input: a4n_api_contract::image_generation::GeneratePageImageInput,
        workflow_run_id: i64,
        progress_reporter: &'a (dyn for<'message> Fn(u32, u32, &'message str) + Send + Sync + 'a),
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Result<a4n_api_contract::image::GeneratedImageSummary, AppError> {
        let host_port =
            crate::actions::image_generation::RuntimeImageGenerationHostPort::new(self.state);
        let progress = ImageGenerationProgress::new(Some(progress_reporter), Some(&cancel_token));
        self.state
            .image_generation_task_application()
            .generate_attached_page_image(user_id, input, workflow_run_id, &host_port, &progress)
            .await
    }
}

pub use a4n_api_contract::manga::{MangaPipelineResumeInput, MangaPipelineStartInput};
pub use execution::preflight_manga_pipeline;
pub use preview::preview_manga_pipeline_requests;

pub async fn start_manga_pipeline_typed(
    state: &AppState,
    user_id: i64,
    input: MangaPipelineStartInput,
) -> Result<GenerationTaskHandle, GenerationTaskStartError> {
    let start = state
        .manga_pipeline_execution_application()
        .start_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }
    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "manga_pipeline",
        move |handle| async move {
            let page_image_generator = RuntimeMangaPipelinePageImageGenerator {
                state: &worker_state,
            };
            worker_state
                .manga_pipeline_execution_application()
                .run_task(user_id, input, handle, &worker_state, &page_image_generator)
                .await
        },
    );
    Ok(start.handle)
}
