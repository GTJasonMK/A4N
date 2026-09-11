#[cfg(test)]
use a4n_db_models::models::ai::generated_video;
#[cfg(test)]
use a4n_db_models::models::{ai::generated_image, media::media_asset};
use a4n_domain::{
    EXTERNAL_IMPORT_MODEL, image::IMAGE_TYPE_CANVAS, video_workflow::WORKFLOW_MODE_CANVAS_VIDEO,
};

const MEDIA_ORIGIN_IMPORTED: &str = "imported";

fn is_freeform_imported_canvas_image_fields(
    image_type: &str,
    model: &str,
    metadata_json: Option<&serde_json::Value>,
) -> bool {
    image_type == IMAGE_TYPE_CANVAS
        && model == EXTERNAL_IMPORT_MODEL
        && metadata_json
            .and_then(|metadata| metadata.get("canvas_import"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

#[cfg(test)]
pub(super) fn is_freeform_imported_canvas_image(image: &generated_image::Model) -> bool {
    is_freeform_imported_canvas_image_fields(
        &image.image_type,
        &image.model,
        image.metadata_json.as_ref(),
    )
}

pub(super) fn is_freeform_imported_canvas_image_projection(
    image_type: &str,
    model: &str,
    metadata_json: Option<&serde_json::Value>,
) -> bool {
    is_freeform_imported_canvas_image_fields(image_type, model, metadata_json)
}

fn is_freeform_imported_canvas_video_fields(
    video_type: &str,
    workflow_mode: Option<&str>,
    model: &str,
    metadata_json: Option<&serde_json::Value>,
) -> bool {
    video_type == a4n_domain::video_workflow::VIDEO_TYPE_CANVAS
        && workflow_mode == Some(WORKFLOW_MODE_CANVAS_VIDEO)
        && model == EXTERNAL_IMPORT_MODEL
        && metadata_json
            .and_then(|metadata| metadata.get("canvas_import"))
            .is_some()
}

#[cfg(test)]
pub(super) fn is_freeform_imported_canvas_video(video: &generated_video::Model) -> bool {
    is_freeform_imported_canvas_video_fields(
        &video.video_type,
        video.workflow_mode.as_deref(),
        &video.model,
        video.metadata_json.as_ref(),
    )
}

pub(super) fn is_freeform_imported_canvas_video_projection(
    video_type: &str,
    workflow_mode: Option<&str>,
    model: &str,
    metadata_json: Option<&serde_json::Value>,
) -> bool {
    is_freeform_imported_canvas_video_fields(video_type, workflow_mode, model, metadata_json)
}

#[cfg(test)]
pub(super) fn is_freeform_generated_media_asset(asset: &media_asset::Model) -> bool {
    is_freeform_generated_media_asset_origin(&asset.origin)
}

pub(super) fn is_freeform_generated_media_asset_origin(origin: &str) -> bool {
    origin != MEDIA_ORIGIN_IMPORTED
}
