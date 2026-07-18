use a4n_core::error::AppError;
use a4n_db::project_dependency_graph::{AssetDependencyGraph, AssetGraphNodeDetail};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn asset_dependency_graph(
        &self,
        project_id: i64,
    ) -> Result<AssetDependencyGraph, AppError> {
        a4n_db::project_dependency_graph::asset_dependency_graph(self.db.connection(), project_id)
            .await
    }

    pub async fn asset_dependency_graph_node_detail(
        &self,
        project_id: i64,
        node_id: &str,
    ) -> Result<AssetGraphNodeDetail, AppError> {
        a4n_db::project_dependency_graph::asset_dependency_graph_node_detail(
            self.db.connection(),
            project_id,
            node_id,
        )
        .await
    }
}
