use super::*;
use a4n_core::state::validator::StateValidator;
use a4n_orchestrator::canvas::validate_canvas_record_viewport_json as validate_canvas_viewport_json;
use a4n_orchestrator::media::{
    MEDIA_STATUS_COMPLETED, is_valid_media_binding_role, is_valid_media_binding_target_kind,
    is_valid_media_kind, is_valid_media_origin, is_valid_media_status,
};
use a4n_orchestrator::project::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM, normalize_project_type};

mod canvas;
mod manifest;
mod media;
mod provenance;
mod rules;

use canvas::{
    CanvasNodeRefIds, validate_canvas_edge_contracts, validate_canvas_node_ref,
    validate_generation_spec_target,
};
#[cfg(test)]
use media::sha256_bytes;
use media::{require_media_payload, validate_optional_file_size};
pub(super) use media::{validate_media_entry_bytes, validate_media_entry_owner};
use provenance::{
    MediaProvenanceSnapshotRefs, validate_media_provenance_snapshot,
    validate_media_provenance_source_external_path,
};
use rules::{parse_canvas_cache_key, validate_unique_ids};

pub(super) fn validate_package_manifest(
    package: &VideoStudioProjectPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    manifest::validate_package_manifest(package, media_payload)
}

pub(super) fn validate_freeform_video_project(
    project: &novel_project::Model,
    label: &str,
) -> Result<(), String> {
    rules::validate_freeform_video_project(project, label)
}

#[cfg(test)]
mod tests;
