use a4n_core::{
    models::{
        ai::{generated_image, generated_video},
        media::media_asset,
    },
    video_workflow::WORKFLOW_MODE_CANVAS_VIDEO,
};
use a4n_domain::{EXTERNAL_IMPORT_MODEL, image::IMAGE_TYPE_CANVAS};

const MEDIA_ORIGIN_IMPORTED: &str = "imported";

pub fn is_freeform_imported_canvas_image(image: &generated_image::Model) -> bool {
    image.image_type == IMAGE_TYPE_CANVAS
        && image.model == EXTERNAL_IMPORT_MODEL
        && image
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas_import"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

pub fn is_freeform_imported_canvas_video(video: &generated_video::Model) -> bool {
    video.video_type == generated_video::VIDEO_TYPE_CANVAS
        && video.workflow_mode.as_deref() == Some(WORKFLOW_MODE_CANVAS_VIDEO)
        && video.model == EXTERNAL_IMPORT_MODEL
        && video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas_import"))
            .is_some()
}

pub fn is_freeform_generated_media_asset(asset: &media_asset::Model) -> bool {
    asset.origin != MEDIA_ORIGIN_IMPORTED
}
