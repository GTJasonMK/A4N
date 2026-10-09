mod contracts;
mod duplicate;
mod errors;
mod execution;
mod import_rows;
mod remap;
mod rules;

use contracts::{DatabaseProjectFamilyImportPackage, ImportIdMaps};
pub use contracts::{
    ProjectFamilyImportMappings, ProjectFamilyImportPackage, ProjectFamilyMediaFileImporter,
};
pub(crate) use duplicate::copy_derived_media_rows;
pub use errors::{ProjectFamilyImportError, ProjectFamilyMediaImportError};
pub use execution::{
    import_project_family_package_rows, import_project_family_package_rows_with_mappings,
};
pub use remap::snapshot_profile_key;
use remap::{
    remap_chapter_hashes_json, remap_chapter_video_reference_image_ids,
    remap_conflict_affected_refs, remap_galgame_presentation_json,
    remap_generation_spec_params_json, remap_generation_spec_source_version_hash,
    remap_manga_generated_image_metadata, remap_manga_page_layout,
    remap_manga_pipeline_artifact_payload, remap_media_request_snapshot,
    remap_media_source_external_path, remap_profile_hashes_json,
    remap_source_snapshot_outline_payload, remap_source_snapshot_profile_payload,
    remap_source_sync_decision_payload, remap_video_metadata,
};
use remap::{
    remap_conflict_decision_json, remap_conflict_local_summary, remap_conflict_source_summary,
    remap_generation_target_id, remap_media_binding_target_id, remap_snapshot_outline_source_id,
    remap_source_sync_target_id, remap_text_anchor_source_id,
};
use rules::media_asset_status_for_import_insert;
pub use rules::normalize_profile_evidence_role;
use rules::{
    map_optional, map_required, remap_chapter_source_revision_key, remap_optional_entity_id,
    remap_profile_id, remap_shooting_script_visual_plan_source, remap_visual_plan_identity,
};

use rules::hash_json;

use a4n_db_models::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    media::media_binding,
    project::novel_project,
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use serde_json::Value;

#[cfg(test)]
use sea_orm::DbErr;

#[cfg(test)]
mod tests;
