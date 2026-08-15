use std::collections::HashMap;

use a4n_core::models::{
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{
        novel_project, profile_evidence, project_source_snapshot, source_snapshot_chapter,
        source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use serde::{Deserialize, Serialize};

use crate::project::ProjectFamilyTextCoreRows;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectFamilyImportPackage {
    pub root_project_id: i64,
    pub projects: Vec<novel_project::Model>,
    pub text_core: ProjectFamilyTextCoreRows,
    pub source_snapshots: Vec<project_source_snapshot::Model>,
    pub source_snapshot_chapters: Vec<source_snapshot_chapter::Model>,
    pub source_snapshot_outlines: Vec<source_snapshot_outline::Model>,
    pub source_snapshot_profiles: Vec<source_snapshot_profile::Model>,
    pub text_anchors: Vec<text_anchor::Model>,
    pub profile_evidence: Vec<profile_evidence::Model>,
    pub generation_specs: Vec<generation_spec::Model>,
    pub generation_spec_text_anchors: Vec<generation_spec_text_anchor::Model>,
    pub generation_spec_entity_refs: Vec<generation_spec_entity_ref::Model>,
    pub generation_spec_media_refs: Vec<generation_spec_media_ref::Model>,
    pub prompt_artifacts: Vec<prompt_artifact::Model>,
    pub media_assets: Vec<media_asset::Model>,
    pub media_provenance: Vec<media_provenance::Model>,
    pub media_bindings: Vec<media_binding::Model>,
    pub galgame_scripts: Vec<galgame_script::Model>,
    pub galgame_scenes: Vec<galgame_scene::Model>,
    pub galgame_lines: Vec<galgame_line::Model>,
    pub galgame_choices: Vec<galgame_choice::Model>,
    pub video_segments: Vec<video_segment::Model>,
    pub assembly_timelines: Vec<assembly_timeline::Model>,
    pub assembly_clips: Vec<assembly_clip::Model>,
    pub source_sync_runs: Vec<source_sync_run::Model>,
    pub source_sync_conflicts: Vec<source_sync_conflict::Model>,
    pub source_sync_decisions: Vec<source_sync_decision::Model>,
}

#[derive(Default)]
pub struct ImportIdMaps {
    pub project_ids: HashMap<i64, i64>,
    pub blueprint_ids: HashMap<i64, i64>,
    pub blueprint_version_ids: HashMap<i64, i64>,
    pub part_outline_ids: HashMap<i64, i64>,
    pub chapter_outline_ids: HashMap<i64, i64>,
    pub chapter_ids: HashMap<i64, i64>,
    pub character_profile_ids: HashMap<i64, i64>,
    pub object_profile_ids: HashMap<i64, i64>,
    pub scene_profile_ids: HashMap<i64, i64>,
    pub scene_occurrence_ids: HashMap<i64, i64>,
    pub scene_transition_ids: HashMap<i64, i64>,
    pub scene_entity_state_ids: HashMap<i64, i64>,
    pub source_snapshot_ids: HashMap<i64, i64>,
    pub source_snapshot_version_hashes: HashMap<i64, String>,
    pub source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub source_snapshot_outline_ids: HashMap<i64, i64>,
    pub source_snapshot_profile_ids: HashMap<i64, i64>,
    pub text_anchor_ids: HashMap<i64, i64>,
    pub profile_evidence_ids: HashMap<i64, i64>,
    pub generation_spec_ids: HashMap<i64, i64>,
    pub prompt_artifact_ids: HashMap<i64, i64>,
    pub media_asset_ids: HashMap<i64, i64>,
    pub media_binding_ids: HashMap<i64, i64>,
    pub galgame_script_ids: HashMap<i64, i64>,
    pub galgame_scene_ids: HashMap<i64, i64>,
    pub galgame_line_ids: HashMap<i64, i64>,
    pub galgame_choice_ids: HashMap<i64, i64>,
    pub video_segment_ids: HashMap<i64, i64>,
    pub assembly_timeline_ids: HashMap<i64, i64>,
    pub assembly_clip_ids: HashMap<i64, i64>,
    pub source_sync_run_ids: HashMap<i64, i64>,
    pub source_sync_conflict_ids: HashMap<i64, i64>,
    pub source_sync_decision_ids: HashMap<i64, i64>,
}

pub trait ProjectFamilyMediaFileImporter {
    fn write_imported_media_field(
        &mut self,
        old_asset: &media_asset::Model,
        new_asset: &media_asset::Model,
        field: &str,
    ) -> Result<Option<String>, super::ProjectFamilyMediaImportError>;
}
