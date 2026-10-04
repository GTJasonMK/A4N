mod export;
mod import_canvas;
mod import_media;
mod import_project;
mod migration;
mod remap;
mod types;

#[cfg(test)]
pub(crate) use types::VideoStudioProjectImportIdMaps;

pub use export::video_studio_project_export_rows;
pub use import_project::import_video_studio_project_record;
pub use migration::migrate_derived_video_canvas_project_record;
pub use types::{
    VideoStudioProjectExportRows, VideoStudioProjectImportError,
    VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter,
    VideoStudioProjectImportRows,
};

pub(super) use crate::project::SOURCE_SYNC_CURRENT;

#[cfg(test)]
use remap::{
    remap_video_studio_canvas_cache_key, remap_video_studio_canvas_metadata,
    remap_video_studio_canvas_node_refs, remap_video_studio_generation_spec_target_entity_id,
};

#[cfg(test)]
mod tests;
