use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{
        novel_project, profile_evidence, project_derivation, project_source_snapshot,
        source_snapshot_chapter, source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_orchestrator::project::ProjectFamilyTextCoreRows;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(super) const PROJECT_FAMILY_PACKAGE_VERSION: &str = "project_family_v1";
pub(super) const MANIFEST_PATH: &str = "a4n-project-family.json";
pub(super) const MEDIA_FILE_FIELD_FILE: &str = "file_path";
pub(super) const MEDIA_FILE_FIELD_THUMBNAIL: &str = "thumbnail_path";
pub(super) const MEDIA_FILE_FIELDS: [(&str, &str); 2] = [
    (MEDIA_FILE_FIELD_FILE, "file"),
    (MEDIA_FILE_FIELD_THUMBNAIL, "thumbnail"),
];

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ProjectFamilyPackage {
    pub(super) version: String,
    pub(super) exported_at: String,
    pub(super) mode: String,
    pub(super) root_project_id: i64,
    pub(super) projects: Vec<novel_project::Model>,
    pub(super) text_core: TextCorePackage,
    pub(super) project_derivations: Vec<project_derivation::Model>,
    pub(super) source_snapshots: Vec<project_source_snapshot::Model>,
    pub(super) source_snapshot_chapters: Vec<source_snapshot_chapter::Model>,
    pub(super) source_snapshot_outlines: Vec<source_snapshot_outline::Model>,
    pub(super) source_snapshot_profiles: Vec<source_snapshot_profile::Model>,
    pub(super) text_anchors: Vec<text_anchor::Model>,
    pub(super) profile_evidence: Vec<profile_evidence::Model>,
    pub(super) generation_specs: Vec<generation_spec::Model>,
    pub(super) generation_spec_text_anchors: Vec<generation_spec_text_anchor::Model>,
    pub(super) generation_spec_entity_refs: Vec<generation_spec_entity_ref::Model>,
    pub(super) generation_spec_media_refs: Vec<generation_spec_media_ref::Model>,
    pub(super) prompt_artifacts: Vec<prompt_artifact::Model>,
    pub(super) media_assets: Vec<media_asset::Model>,
    pub(super) media_provenance: Vec<media_provenance::Model>,
    pub(super) media_bindings: Vec<media_binding::Model>,
    pub(super) galgame_scripts: Vec<galgame_script::Model>,
    pub(super) galgame_scenes: Vec<galgame_scene::Model>,
    pub(super) galgame_lines: Vec<galgame_line::Model>,
    pub(super) galgame_choices: Vec<galgame_choice::Model>,
    pub(super) video_segments: Vec<video_segment::Model>,
    pub(super) assembly_timelines: Vec<assembly_timeline::Model>,
    pub(super) assembly_clips: Vec<assembly_clip::Model>,
    pub(super) source_sync_runs: Vec<source_sync_run::Model>,
    pub(super) source_sync_conflicts: Vec<source_sync_conflict::Model>,
    pub(super) source_sync_decisions: Vec<source_sync_decision::Model>,
    pub(super) media_files: Vec<MediaFileEntry>,
}

pub(super) type TextCorePackage = ProjectFamilyTextCoreRows;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MediaFileEntry {
    pub(super) media_asset_id: i64,
    pub(super) field: String,
    pub(super) original_path: String,
    pub(super) package_path: String,
}

pub(super) struct ImportMediaPayload {
    pub(super) entries: HashMap<(i64, String), MediaFileEntry>,
    pub(super) bytes: HashMap<String, Vec<u8>>,
}
