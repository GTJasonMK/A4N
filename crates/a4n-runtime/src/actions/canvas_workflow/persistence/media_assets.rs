use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::media::media_asset;
use a4n_orchestrator::canvas::{
    CanvasImageMediaAssetEnsureInput, CanvasVideoMediaAssetEnsureInput,
};

pub(in crate::actions::canvas_workflow) async fn ensure_canvas_image_media_asset(
    state: &AppState,
    project_id: i64,
    image: &generated_image::Model,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<media_asset::Model, String> {
    state
        .orchestrator
        .ensure_canvas_image_media_asset_record(CanvasImageMediaAssetEnsureInput {
            project_id,
            image: image.clone(),
            generation_spec_id,
            origin: origin.to_string(),
        })
        .await
        .map_err(app_error_message)
}

pub(in crate::actions::canvas_workflow) async fn ensure_canvas_video_media_asset(
    state: &AppState,
    project_id: i64,
    video: &generated_video::Model,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<media_asset::Model, String> {
    state
        .orchestrator
        .ensure_canvas_video_media_asset_record(CanvasVideoMediaAssetEnsureInput {
            project_id,
            video: video.clone(),
            generation_spec_id,
            origin: origin.to_string(),
        })
        .await
        .map_err(app_error_message)
}
