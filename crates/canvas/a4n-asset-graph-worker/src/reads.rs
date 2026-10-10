use super::*;

impl WorkerState {
    pub(super) async fn graph(
        &self,
        request: WorkerEnvelope<ProjectAssetGraphInput>,
    ) -> Result<a4n_asset_graph_worker_contract::AssetDependencyGraph, WorkerFailure> {
        validate_request(&request, ASSET_GRAPH_OPERATION)?;
        request.require_project_scope(request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        self.verify_permission(request.payload.project_id, user_id)
            .await?;
        a4n_db::project::dependency_graph::asset_dependency_graph(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(app_error)
    }

    pub(super) async fn node_detail(
        &self,
        request: WorkerEnvelope<ProjectAssetGraphNodeDetailInput>,
    ) -> Result<a4n_asset_graph_worker_contract::AssetGraphNodeDetail, WorkerFailure> {
        validate_request(&request, ASSET_GRAPH_NODE_DETAIL_OPERATION)?;
        request.require_project_scope(request.payload.project_id)?;
        if request.payload.node_id.trim().is_empty() {
            return Err(WorkerFailure::invalid_request("node_id 不能为空"));
        }
        let user_id = request.principal.require_user_id()?;
        self.verify_permission(request.payload.project_id, user_id)
            .await?;
        a4n_db::project::dependency_graph::asset_dependency_graph_node_detail(
            self.database.connection(),
            request.payload.project_id,
            request.payload.node_id.trim(),
        )
        .await
        .map_err(app_error)
    }
}
