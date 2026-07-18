use a4n_core::models::{
    ai::{generated_image, generated_video},
    media::media_asset,
};

const EXTERNAL_IMPORT_MODEL: &str = "external-import";
const CANVAS_IMAGE_TYPE: &str = "canvas_image";
const CANVAS_VIDEO_TYPE: &str = "canvas_video";
const MEDIA_ORIGIN_IMPORTED: &str = "imported";

pub fn is_freeform_imported_canvas_image(image: &generated_image::Model) -> bool {
    image.image_type == CANVAS_IMAGE_TYPE
        && image.model == EXTERNAL_IMPORT_MODEL
        && image
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas_import"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

pub fn is_freeform_imported_canvas_video(video: &generated_video::Model) -> bool {
    video.video_type == CANVAS_VIDEO_TYPE
        && video.workflow_mode.as_deref() == Some(CANVAS_VIDEO_TYPE)
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
