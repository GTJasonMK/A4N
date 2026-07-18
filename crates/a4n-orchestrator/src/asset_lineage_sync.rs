pub use a4n_db::asset_lineage_sync::{AssetLineageBackfillSummary, AssetLineageSyncError};

use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn sync_project_asset_lineage(
        &self,
        project_id: i64,
    ) -> Result<AssetLineageBackfillSummary, AppError> {
        a4n_db::asset_lineage_sync::sync_project_asset_lineage_record(
            self.db.connection(),
            project_id,
        )
        .await
        .map_err(asset_lineage_sync_error)
    }
}

pub(crate) fn asset_lineage_sync_error(error: AssetLineageSyncError) -> AppError {
    match error {
        AssetLineageSyncError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AssetLineageSyncError::NotFound(message) => AppError::NotFound(message),
        AssetLineageSyncError::Database(error) => AppError::Database(error),
    }
}
