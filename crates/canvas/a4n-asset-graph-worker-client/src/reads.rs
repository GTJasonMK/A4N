use a4n_api_contract::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, ProjectAssetGraphApplicationPort,
};
use a4n_asset_graph_worker_contract::{
    ASSET_GRAPH_NODE_DETAIL_OPERATION, ASSET_GRAPH_OPERATION, ProjectAssetGraphInput,
    ProjectAssetGraphNodeDetailInput,
};
use a4n_error::AppError;

use super::AssetGraphWorkerClient;

#[async_trait::async_trait]
impl ProjectAssetGraphApplicationPort for AssetGraphWorkerClient {
    async fn graph(&self, user_id: i64, project_id: i64) -> Result<AssetDependencyGraph, AppError> {
        self.command(
            user_id,
            project_id,
            ASSET_GRAPH_OPERATION,
            ProjectAssetGraphInput { project_id },
        )
        .await
    }

    async fn node_detail(
        &self,
        user_id: i64,
        project_id: i64,
        node_id: String,
    ) -> Result<AssetGraphNodeDetail, AppError> {
        self.command(
            user_id,
            project_id,
            ASSET_GRAPH_NODE_DETAIL_OPERATION,
            ProjectAssetGraphNodeDetailInput {
                project_id,
                node_id,
            },
        )
        .await
    }
}
