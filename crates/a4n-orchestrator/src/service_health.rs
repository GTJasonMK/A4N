pub(super) mod contracts;
mod errors;
mod summary;

pub(super) use summary::service_health_summary;

/// Runtime-facing managed-service health boundary.
#[async_trait::async_trait]
pub trait ServiceHealthApplicationPort: Send + Sync {
    async fn summaries(&self) -> Result<Vec<ManagedServiceHealthSummary>, a4n_error::AppError>;
}

pub use contracts::ManagedServiceHealthSummary;
