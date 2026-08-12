mod context;
mod evidence;
mod execution;
mod parsing;
mod preview;
mod progress;
mod stages;
mod types;
mod workflow;

use a4n_core::error::AppError;
use async_trait::async_trait;

pub use progress::{
    SCENE_PROFILE_SYNC_PROGRESS_TOTAL, SceneProfileSyncProgress, SceneProfileSyncProgressReporter,
};
pub use types::SceneProfileSyncPreviewStep;

#[async_trait]
pub trait SceneProfileSyncHostPort: Send + Sync {
    async fn sync_scene_rag_records(
        &self,
        project_id: i64,
        workflow_run_id: i64,
    ) -> Result<(), AppError>;
}

#[cfg(test)]
mod tests;
