//! 视频画布项目包纯校验子树。
//!
//! 下列导入供子模块通过 `use super::*` / `use super::super::*` 共享。

#![allow(unused_imports)]

use std::collections::{HashMap, HashSet};

use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::novel_project,
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_core::state::validator::StateValidator;

use crate::canvas::validate_canvas_record_viewport_json as validate_canvas_viewport_json;
use crate::media::{
    MEDIA_STATUS_COMPLETED, is_valid_media_binding_role, is_valid_media_binding_target_kind,
    is_valid_media_kind, is_valid_media_origin, is_valid_media_status,
};
#[cfg(test)]
use crate::project::PROJECT_KIND_VIDEO;
#[cfg(test)]
use crate::project::sha256_bytes;
use crate::project::{SOURCE_MODE_FREEFORM, SOURCE_SYNC_CURRENT, normalize_project_type};
use crate::project::{parse_canvas_cache_key, validate_unique_ids};

use super::{
    ImportMediaPayload, MEDIA_FIELD_FILE, MEDIA_FIELD_THUMBNAIL, MEDIA_OWNER_IMAGE,
    MEDIA_OWNER_MEDIA_ASSET, MEDIA_OWNER_VIDEO, PACKAGE_MODE, VIDEO_STUDIO_PACKAGE_VERSION,
    VideoStudioMediaFileEntry, VideoStudioProjectPackage, media_file_key,
};

mod canvas;
mod manifest;
mod media;
mod provenance;

use canvas::{
    CanvasNodeRefIds, validate_canvas_edge_contracts, validate_canvas_node_ref,
    validate_generation_spec_target,
};
use media::{require_media_payload, validate_optional_file_size};
pub(crate) use media::{validate_media_entry_bytes, validate_media_entry_owner};
use provenance::{
    MediaProvenanceSnapshotRefs, validate_media_provenance_snapshot,
    validate_media_provenance_source_external_path,
};

pub(crate) fn validate_package_manifest(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    manifest::validate_package_manifest(package, media_payload)
}

#[cfg(test)]
mod tests;
