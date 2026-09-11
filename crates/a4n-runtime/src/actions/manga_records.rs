mod optimization;
mod pipeline;
mod records;

use crate::{AppState, actions::dependency_conflict::delete_downstream_images_if_requested};
use a4n_error::AppError;
use a4n_orchestrator::manga::MangaPromptDownstreamCleanupPort;
use async_trait::async_trait;

pub(super) struct RuntimeMangaDownstreamCleanup<'a> {
    pub(super) state: &'a AppState,
    pub(super) user_id: i64,
}

#[async_trait]
impl MangaPromptDownstreamCleanupPort for RuntimeMangaDownstreamCleanup<'_> {
    async fn delete_downstream_images(&self, image_ids: &[i64]) -> Result<(), AppError> {
        delete_downstream_images_if_requested(self.state, self.user_id, image_ids.iter().copied())
            .await
            .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
    }
}

pub use optimization::optimize_manga_prompt;
pub use pipeline::{
    cancel_manga_pipeline, get_manga_pipeline_status, list_manga_pipeline_statuses,
};
pub use records::{
    bind_manga_reference_image, delete_manga_by_chapter, delete_manga_prompt,
    list_manga_page_prompts, update_manga_prompt,
};

#[cfg(test)]
mod tests;
