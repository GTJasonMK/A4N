use std::collections::HashMap;

use a4n_core::models::project::novel_project;

#[derive(Debug, Clone, Default)]
pub(in crate::project::duplicate::freeform) struct FreeformGeneratedImageCopyRecords {
    pub(super) ids: HashMap<i64, i64>,
    pub(super) file_paths: HashMap<i64, String>,
}

#[derive(Debug, Clone, Default)]
pub(in crate::project::duplicate::freeform) struct FreeformGeneratedVideoCopyRecords {
    pub(super) ids: HashMap<i64, i64>,
    pub(super) file_paths: HashMap<i64, Option<String>>,
    pub(super) thumbnail_paths: HashMap<i64, Option<String>>,
}

#[derive(Debug, Clone)]
pub(in crate::project::duplicate::freeform) struct FreeformProjectMediaCopyResult {
    pub(super) project: novel_project::Model,
    pub(super) images: FreeformGeneratedImageCopyRecords,
    pub(super) videos: FreeformGeneratedVideoCopyRecords,
}

#[derive(Debug, Clone, Default)]
pub(in crate::project::duplicate::freeform) struct FreeformProjectMediaGraphCopyResult {
    pub(super) media_asset_ids: HashMap<i64, i64>,
    pub(super) video_segment_ids: HashMap<i64, i64>,
    pub(super) assembly_timeline_ids: HashMap<i64, i64>,
    pub(super) assembly_clip_ids: HashMap<i64, i64>,
}

#[derive(Clone, Default)]
pub(in crate::project::duplicate::freeform) struct FreeformCopyMaps {
    pub(super) images: FreeformGeneratedImageCopyRecords,
    pub(super) videos: FreeformGeneratedVideoCopyRecords,
    pub(super) text_asset_ids: HashMap<i64, i64>,
    pub(super) graph_ids: HashMap<i64, i64>,
    pub(super) node_ids: HashMap<i64, i64>,
    pub(super) generation_spec_ids: HashMap<i64, i64>,
    pub(super) prompt_artifact_ids: HashMap<i64, i64>,
    pub(super) media_asset_ids: HashMap<i64, i64>,
    pub(super) video_segment_ids: HashMap<i64, i64>,
    pub(super) assembly_timeline_ids: HashMap<i64, i64>,
    pub(super) assembly_clip_ids: HashMap<i64, i64>,
}

#[derive(Default)]
pub(in crate::project::duplicate::freeform) struct FreeformCanvasCopyMaps {
    pub(super) graph_ids: HashMap<i64, i64>,
    pub(super) node_ids: HashMap<i64, i64>,
}

pub(in crate::project::duplicate::freeform) struct CopiedFreeformGenerationSpecs {
    pub(super) spec_ids: Vec<i64>,
    pub(super) spec_id_map: HashMap<i64, i64>,
    pub(super) deferred_targets: Vec<DeferredFreeformSpecTarget>,
}

pub(in crate::project::duplicate::freeform) struct DeferredFreeformSpecTarget {
    pub(super) new_spec_id: i64,
    pub(super) target_entity_kind: String,
    pub(super) old_target_entity_id: i64,
}
