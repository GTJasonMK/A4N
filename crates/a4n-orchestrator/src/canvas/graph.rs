use a4n_core::error::AppError;

use super::{CanvasGraphRecordDocument, CanvasGraphRecordInput, CanvasTextAssetRecordInput};
use crate::OrchestratorContext;
use crate::canvas::errors::canvas_db_error;

impl OrchestratorContext {
    pub async fn get_or_create_canvas_graph_record(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
        now: chrono::NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        a4n_db::canvas::get_or_create_canvas_graph_record(
            self.db.connection(),
            project_id,
            graph_id,
            now,
        )
        .await
        .map_err(canvas_db_error)
    }

    pub async fn save_canvas_graph_record(
        &self,
        project_id: i64,
        input: CanvasGraphRecordInput,
        now: chrono::NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        a4n_db::canvas::save_canvas_graph_record(self.db.connection(), project_id, input, now)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn upsert_canvas_text_asset_record(
        &self,
        project_id: i64,
        input: CanvasTextAssetRecordInput,
        now: chrono::NaiveDateTime,
    ) -> Result<a4n_core::models::canvas::canvas_text_asset::Model, AppError> {
        a4n_db::canvas::upsert_canvas_text_asset_record(
            self.db.connection(),
            project_id,
            input,
            now,
        )
        .await
        .map_err(canvas_db_error)
    }
}
