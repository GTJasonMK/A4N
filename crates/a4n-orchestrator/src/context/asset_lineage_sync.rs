//! 资产谱系同步领域私有 owner。
//!
//! 持有 `DbHandle`，封装项目资产谱系 backfill/sync。

use a4n_core::error::AppError;
use a4n_db::DbHandle;
use a4n_db::asset_lineage_sync::{AssetLineageBackfillSummary, AssetLineageSyncError};

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct AssetLineageSyncApplication {
    database: DbHandle,
}

impl AssetLineageSyncApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn sync_project_asset_lineage(
        &self,
        project_id: i64,
    ) -> Result<AssetLineageBackfillSummary, AppError> {
        a4n_db::asset_lineage_sync::sync_project_asset_lineage_record(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(asset_lineage_sync_error)
    }
}

fn asset_lineage_sync_error(error: AssetLineageSyncError) -> AppError {
    match error {
        AssetLineageSyncError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AssetLineageSyncError::NotFound(message) => AppError::NotFound(message),
        AssetLineageSyncError::Database(error) => AppError::Database(error),
    }
}

impl OrchestratorContext {
    pub async fn sync_project_asset_lineage(
        &self,
        project_id: i64,
    ) -> Result<AssetLineageBackfillSummary, AppError> {
        self.asset_lineage_sync
            .sync_project_asset_lineage(project_id)
            .await
    }
}
