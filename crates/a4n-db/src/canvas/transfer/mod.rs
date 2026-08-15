mod export;
mod import_canvas;
mod import_media;
mod import_project;
mod remap;
mod types;

pub use export::video_studio_project_export_rows;
pub use import_project::import_video_studio_project_record;
pub use types::{
    VideoStudioProjectExportRows, VideoStudioProjectImportError,
    VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter,
    VideoStudioProjectImportRows,
};

pub(super) use crate::project::SOURCE_SYNC_CURRENT;
pub(super) const MEDIA_OWNER_IMAGE: &str = "generated_image";
pub(super) const MEDIA_OWNER_VIDEO: &str = "generated_video";
pub(super) const MEDIA_OWNER_MEDIA_ASSET: &str = "media_asset";
pub(super) const MEDIA_FIELD_FILE: &str = "file_path";
pub(super) const MEDIA_FIELD_THUMBNAIL: &str = "thumbnail_path";

#[cfg(test)]
use remap::{
    remap_video_studio_canvas_cache_key, remap_video_studio_canvas_metadata,
    remap_video_studio_canvas_node_refs, remap_video_studio_generation_spec_target_entity_id,
};
#[cfg(test)]
use types::VideoStudioProjectImportIdMaps;

#[cfg(test)]
mod tests;
