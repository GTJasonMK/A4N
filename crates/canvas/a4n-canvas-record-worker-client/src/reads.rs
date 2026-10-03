use super::*;

#[async_trait::async_trait]
impl CanvasReadApplicationPort for CanvasRecordWorkerClient {
    async fn get_graph(
        &self,
        user_id: i64,
        input: GetCanvasGraphInput,
    ) -> Result<CanvasGraphDocument, AppError> {
        self.command(user_id, input.project_id, CANVAS_GRAPH_GET_OPERATION, input)
            .await
    }

    async fn read_graph(
        &self,
        user_id: i64,
        input: ReadCanvasGraphInput,
    ) -> Result<CanvasGraphReadResult, AppError> {
        self.command(
            user_id,
            input.project_id,
            CANVAS_GRAPH_READ_OPERATION,
            input,
        )
        .await
    }

    async fn list_text_assets(
        &self,
        user_id: i64,
        input: ListCanvasTextAssetsInput,
    ) -> Result<Vec<CanvasTextAssetSummary>, AppError> {
        self.command(
            user_id,
            input.project_id,
            CANVAS_TEXT_ASSET_LIST_OPERATION,
            input,
        )
        .await
    }

    async fn list_text_asset_versions(
        &self,
        user_id: i64,
        input: ListCanvasTextAssetVersionsInput,
    ) -> Result<Vec<CanvasTextAssetVersionSummary>, AppError> {
        self.command(
            user_id,
            input.project_id,
            CANVAS_TEXT_ASSET_VERSIONS_OPERATION,
            input,
        )
        .await
    }
}
