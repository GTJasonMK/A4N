use crate::file_utils::canonicalize_data_file_path;
use crate::state::AppState;
use a4n_core::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, part_outline},
    character::protagonist_profile,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        novel_project, profile_evidence, project_derivation, project_source_snapshot,
        source_snapshot_chapter, source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_core::state::validator::StateValidator;
use a4n_services::{
    generation::{
        is_valid_generation_entity_kind, is_valid_generation_entity_role,
        is_valid_generation_media_ref_role, is_valid_generation_prompt_kind,
        is_valid_generation_spec_status, is_valid_generation_spec_target_entity_kind,
        is_valid_generation_spec_target_kind, is_valid_generation_text_anchor_role,
    },
    media::{
        MEDIA_STATUS_COMPLETED, MEDIA_STATUS_PENDING, galgame_media_binding_required_media_kind,
        is_valid_galgame_media_binding_pair, is_valid_media_binding_role,
        is_valid_media_binding_target_kind, is_valid_media_kind, is_valid_media_origin,
        is_valid_media_status,
    },
    project::{
        PROJECT_KIND_TEXT, SOURCE_SYNC_CURRENT, is_valid_derived_project_status,
        is_valid_source_sync_status, normalize_derived_project_kind, normalize_project_kind,
        normalize_project_type,
    },
    source_sync::{
        CONFLICT_STATUS_IGNORED, CONFLICT_STATUS_PENDING, CONFLICT_STATUS_RESOLVED,
        SYNC_RUN_MODE_APPLY, SYNC_RUN_MODE_DRY_RUN, SYNC_RUN_STATUS_APPLYING,
        SYNC_RUN_STATUS_COMPLETED, SYNC_RUN_STATUS_PLANNED, is_valid_conflict_severity,
        is_valid_conflict_status, is_valid_decision_actor, is_valid_local_state_kind,
        is_valid_resolution_strategy, is_valid_source_change_kind, is_valid_sync_run_mode,
        is_valid_sync_run_status,
    },
    text_anchor::{
        EVIDENCE_ROLE_APPEARANCE, EVIDENCE_ROLE_BEHAVIOR, EVIDENCE_ROLE_FIRST_SEEN,
        EVIDENCE_ROLE_LOCATION, EVIDENCE_ROLE_RELATIONSHIP, EVIDENCE_ROLE_STATE_CHANGE,
        SOURCE_KIND_CHAPTER_CONTENT,
    },
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, NotSet, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    io::{Cursor, Read, Seek, Write},
    path::{Path, PathBuf},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const PROJECT_FAMILY_PACKAGE_VERSION: &str = "project_family_v1";
const MANIFEST_PATH: &str = "a4n-project-family.json";
const MEDIA_FILE_FIELD_FILE: &str = "file_path";
const MEDIA_FILE_FIELD_THUMBNAIL: &str = "thumbnail_path";
const MEDIA_FILE_FIELDS: [(&str, &str); 2] = [
    (MEDIA_FILE_FIELD_FILE, "file"),
    (MEDIA_FILE_FIELD_THUMBNAIL, "thumbnail"),
];

#[derive(Debug, Serialize, Deserialize)]
struct ProjectFamilyPackage {
    version: String,
    exported_at: String,
    mode: String,
    root_project_id: i64,
    projects: Vec<novel_project::Model>,
    text_core: TextCorePackage,
    project_derivations: Vec<project_derivation::Model>,
    source_snapshots: Vec<project_source_snapshot::Model>,
    source_snapshot_chapters: Vec<source_snapshot_chapter::Model>,
    source_snapshot_outlines: Vec<source_snapshot_outline::Model>,
    source_snapshot_profiles: Vec<source_snapshot_profile::Model>,
    text_anchors: Vec<text_anchor::Model>,
    profile_evidence: Vec<profile_evidence::Model>,
    generation_specs: Vec<generation_spec::Model>,
    generation_spec_text_anchors: Vec<generation_spec_text_anchor::Model>,
    generation_spec_entity_refs: Vec<generation_spec_entity_ref::Model>,
    generation_spec_media_refs: Vec<generation_spec_media_ref::Model>,
    prompt_artifacts: Vec<prompt_artifact::Model>,
    media_assets: Vec<media_asset::Model>,
    media_provenance: Vec<media_provenance::Model>,
    media_bindings: Vec<media_binding::Model>,
    galgame_scripts: Vec<galgame_script::Model>,
    galgame_scenes: Vec<galgame_scene::Model>,
    galgame_lines: Vec<galgame_line::Model>,
    galgame_choices: Vec<galgame_choice::Model>,
    video_segments: Vec<video_segment::Model>,
    assembly_timelines: Vec<assembly_timeline::Model>,
    assembly_clips: Vec<assembly_clip::Model>,
    source_sync_runs: Vec<source_sync_run::Model>,
    source_sync_conflicts: Vec<source_sync_conflict::Model>,
    source_sync_decisions: Vec<source_sync_decision::Model>,
    media_files: Vec<MediaFileEntry>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TextCorePackage {
    blueprint: Option<novel_blueprint::Model>,
    blueprint_versions: Vec<blueprint_version::Model>,
    blueprint_characters: Vec<blueprint_character::Model>,
    blueprint_relationships: Vec<blueprint_relationship::Model>,
    part_outlines: Vec<part_outline::Model>,
    chapter_outlines: Vec<chapter_outline::Model>,
    chapters: Vec<chapter::Model>,
    character_profiles: Vec<protagonist_profile::Model>,
    object_profiles: Vec<object_profile::Model>,
    scene_profiles: Vec<scene_profile::Model>,
    scene_occurrences: Vec<scene_occurrence::Model>,
    scene_transitions: Vec<scene_transition::Model>,
    scene_entity_states: Vec<scene_entity_state::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MediaFileEntry {
    media_asset_id: i64,
    field: String,
    original_path: String,
    package_path: String,
}

#[derive(Default)]
struct ImportIdMaps {
    project_ids: HashMap<i64, i64>,
    blueprint_ids: HashMap<i64, i64>,
    blueprint_version_ids: HashMap<i64, i64>,
    part_outline_ids: HashMap<i64, i64>,
    chapter_outline_ids: HashMap<i64, i64>,
    chapter_ids: HashMap<i64, i64>,
    character_profile_ids: HashMap<i64, i64>,
    object_profile_ids: HashMap<i64, i64>,
    scene_profile_ids: HashMap<i64, i64>,
    scene_occurrence_ids: HashMap<i64, i64>,
    scene_transition_ids: HashMap<i64, i64>,
    scene_entity_state_ids: HashMap<i64, i64>,
    source_snapshot_ids: HashMap<i64, i64>,
    source_snapshot_version_hashes: HashMap<i64, String>,
    source_snapshot_chapter_ids: HashMap<i64, i64>,
    source_snapshot_outline_ids: HashMap<i64, i64>,
    source_snapshot_profile_ids: HashMap<i64, i64>,
    text_anchor_ids: HashMap<i64, i64>,
    profile_evidence_ids: HashMap<i64, i64>,
    generation_spec_ids: HashMap<i64, i64>,
    prompt_artifact_ids: HashMap<i64, i64>,
    media_asset_ids: HashMap<i64, i64>,
    media_binding_ids: HashMap<i64, i64>,
    galgame_script_ids: HashMap<i64, i64>,
    galgame_scene_ids: HashMap<i64, i64>,
    galgame_line_ids: HashMap<i64, i64>,
    galgame_choice_ids: HashMap<i64, i64>,
    video_segment_ids: HashMap<i64, i64>,
    assembly_timeline_ids: HashMap<i64, i64>,
    assembly_clip_ids: HashMap<i64, i64>,
    source_sync_run_ids: HashMap<i64, i64>,
    source_sync_conflict_ids: HashMap<i64, i64>,
    source_sync_decision_ids: HashMap<i64, i64>,
}

struct ImportMediaPayload {
    entries: HashMap<(i64, String), MediaFileEntry>,
    bytes: HashMap<String, Vec<u8>>,
}

mod archive;
mod export;
mod import_rows;
mod remap;
mod validation;

pub(super) async fn export_project_family_package_bytes(
    state: &AppState,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<Vec<u8>, String> {
    export::export_project_family_package_bytes(state, root_project_id, include_derived_projects)
        .await
}

pub(super) async fn export_derived_project_package_bytes(
    state: &AppState,
    derived_project_id: i64,
) -> Result<Vec<u8>, String> {
    export::export_derived_project_package_bytes(state, derived_project_id).await
}

pub(super) async fn import_project_family_package_bytes(
    state: &AppState,
    user_id: i64,
    bytes: Vec<u8>,
) -> Result<Vec<novel_project::Model>, String> {
    archive::import_project_family_package_bytes(state, user_id, bytes).await
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    sha256_bytes(&bytes)
}

fn hash_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

fn media_asset_path<'a>(
    asset: &'a media_asset::Model,
    field: &str,
) -> Result<Option<&'a str>, String> {
    match field {
        MEDIA_FILE_FIELD_FILE => Ok(asset.file_path.as_deref()),
        MEDIA_FILE_FIELD_THUMBNAIL => Ok(asset.thumbnail_path.as_deref()),
        _ => Err(format!("媒体文件字段无效: {field}")),
    }
}

fn media_file_key(media_asset_id: i64, field: &str) -> (i64, String) {
    (media_asset_id, field.to_string())
}

fn required_package_extension(path: &str) -> Result<String, String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("媒体包路径缺少扩展名: {path}"))
}

#[cfg(test)]
mod tests;
