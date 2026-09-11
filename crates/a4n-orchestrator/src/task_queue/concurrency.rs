use a4n_db::SharedDatabaseConnection;
use a4n_error::AppError;

use super::queue_db_error;

#[cfg(test)]
mod tests;

pub(super) struct QueueConcurrencyLimits {
    pub(super) llm: usize,
    pub(super) image: usize,
    pub(super) video: usize,
    pub(super) embedding: usize,
    pub(super) background: usize,
}

impl QueueConcurrencyLimits {
    pub(super) async fn load(db: &SharedDatabaseConnection) -> Result<Self, AppError> {
        let provider_limits = a4n_db::queue::load_default_provider_concurrency_limits(db.as_ref())
            .await
            .map_err(queue_db_error)?;
        Ok(Self {
            llm: provider_limits.llm,
            image: provider_limits.image,
            video: provider_limits.video,
            embedding: 2,
            background: 2,
        })
    }
}
