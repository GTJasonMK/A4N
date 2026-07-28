use a4n_core::error::AppError;
use a4n_db::{
    DbHandle,
    project_dependency_graph::{AssetDependencyGraph, AssetGraphNodeDetail},
};

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct ProjectDependencyGraphApplication {
    database: DbHandle,
}

impl ProjectDependencyGraphApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn graph(&self, project_id: i64) -> Result<AssetDependencyGraph, AppError> {
        a4n_db::project_dependency_graph::asset_dependency_graph(
            self.database.connection(),
            project_id,
        )
        .await
    }

    async fn node_detail(
        &self,
        project_id: i64,
        node_id: &str,
    ) -> Result<AssetGraphNodeDetail, AppError> {
        a4n_db::project_dependency_graph::asset_dependency_graph_node_detail(
            self.database.connection(),
            project_id,
            node_id,
        )
        .await
    }
}

impl OrchestratorContext {
    pub async fn asset_dependency_graph(
        &self,
        project_id: i64,
    ) -> Result<AssetDependencyGraph, AppError> {
        self.project_dependency_graph.graph(project_id).await
    }

    pub async fn asset_dependency_graph_node_detail(
        &self,
        project_id: i64,
        node_id: &str,
    ) -> Result<AssetGraphNodeDetail, AppError> {
        self.project_dependency_graph
            .node_detail(project_id, node_id)
            .await
    }
}
