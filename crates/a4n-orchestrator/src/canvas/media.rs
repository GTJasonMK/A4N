use a4n_core::error::AppError;
use a4n_core::models::media::media_asset;

use super::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary,
    CanvasImageMediaAssetEnsureInput, CanvasVideoMediaAssetEnsureInput,
};
use crate::OrchestratorContext;
use crate::canvas::errors::canvas_db_error;

impl OrchestratorContext {
    pub async fn ensure_canvas_image_media_asset_record(
        &self,
        input: CanvasImageMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        a4n_db::canvas::ensure_canvas_image_media_asset_record(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn ensure_canvas_video_media_asset_record(
        &self,
        input: CanvasVideoMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        a4n_db::canvas::ensure_canvas_video_media_asset_record(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn sync_canvas_assembly_contract_records(
        &self,
        input: CanvasAssemblyContractSyncInput,
    ) -> Result<CanvasAssemblyContractSyncSummary, AppError> {
        a4n_db::canvas::sync_canvas_assembly_contract_records(self.db.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    pub async fn canvas_media_asset_by_id(
        &self,
        media_asset_id: i64,
    ) -> Result<Option<media_asset::Model>, AppError> {
        a4n_db::canvas::canvas_media_asset_by_id(self.db.connection(), media_asset_id)
            .await
            .map_err(canvas_db_error)
    }
}
