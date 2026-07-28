mod contracts;
mod optimization;
mod pipeline;
mod projections;
mod records;

use crate::{AppState, actions::dependency_conflict::delete_downstream_images_if_requested};
use a4n_core::error::AppError;
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

pub use contracts::*;
pub use optimization::optimize_manga_prompt;
pub use pipeline::*;
pub use records::*;

#[cfg(test)]
mod tests;
