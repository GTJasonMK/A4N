//! 资产谱系同步 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_orchestrator::{OrchestratorContext, asset_lineage_sync::AssetLineageBackfillSummary};

pub(crate) struct AssetLineageSyncRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> AssetLineageSyncRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn sync_project_asset_lineage(
        &self,
        project_id: i64,
    ) -> Result<AssetLineageBackfillSummary, AppError> {
        self.orchestrator
            .sync_project_asset_lineage(project_id)
            .await
    }
}
