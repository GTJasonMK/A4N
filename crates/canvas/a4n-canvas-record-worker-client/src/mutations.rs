use super::*;

#[async_trait::async_trait]
impl CanvasMutationApplicationPort for CanvasRecordWorkerClient {
    async fn save_graph(
        &self,
        user_id: i64,
        input: SaveCanvasGraphInput,
    ) -> Result<CanvasGraphSaveResult, AppError> {
        self.command(
            user_id,
            input.project_id,
            CANVAS_GRAPH_SAVE_OPERATION,
            input,
        )
        .await
    }

    async fn edit_graph(
        &self,
        user_id: i64,
        input: EditCanvasGraphInput,
    ) -> Result<CanvasGraphEditResult, AppError> {
        self.command(
            user_id,
            input.project_id,
            CANVAS_GRAPH_EDIT_OPERATION,
            input,
        )
        .await
    }

    async fn save_text_asset(
        &self,
        user_id: i64,
        input: SaveCanvasTextAssetInput,
    ) -> Result<CanvasTextAssetSummary, AppError> {
        self.command(
            user_id,
            input.project_id,
            CANVAS_TEXT_ASSET_SAVE_OPERATION,
            input,
        )
        .await
    }
}
