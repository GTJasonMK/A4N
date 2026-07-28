use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    project::{AssetDependencyGraph, AssetGraphNodeDetail},
};

pub(crate) struct ProjectDependencyGraphRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectDependencyGraphRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn asset_dependency_graph(
        &self,
        project_id: i64,
    ) -> Result<AssetDependencyGraph, AppError> {
        self.orchestrator.asset_dependency_graph(project_id).await
    }

    pub(crate) async fn asset_dependency_graph_node_detail(
        &self,
        project_id: i64,
        node_id: &str,
    ) -> Result<AssetGraphNodeDetail, AppError> {
        self.orchestrator
            .asset_dependency_graph_node_detail(project_id, node_id)
            .await
    }
}
