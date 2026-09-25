mod export;
mod import_canvas;
mod import_media;
mod import_project;
mod remap;
mod types;

pub(crate) use import_canvas::{
    import_video_studio_canvas_graphs, import_video_studio_canvas_text_assets,
    import_video_studio_generation_specs, update_imported_generation_spec_metadata,
    update_imported_generation_spec_targets,
};
pub(crate) use import_media::{update_imported_canvas_node_refs, update_imported_media_metadata};
pub(crate) use types::{
    DeferredVideoStudioImportGenerationSpecTarget, VideoStudioProjectImportIdMaps,
};

pub use export::video_studio_project_export_rows;
pub use import_project::import_video_studio_project_record;
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
