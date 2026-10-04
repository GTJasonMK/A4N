use std::collections::HashMap;

use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    asset::object_profile,
    chapter::{
        chapter, chapter_manga_prompt, chapter_outline, chapter_shooting_script,
        chapter_visual_plan, part_outline,
    },
    character::{character_portrait, protagonist_profile},
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::{media_asset, media_binding, media_provenance},
    project::{
        blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
        profile_evidence, project_source_snapshot, source_snapshot_chapter,
        source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_domain::project::*;
use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use serde::{Deserialize, Serialize};

pub const PROJECT_FAMILY_MEDIA_OWNER_CHARACTER_PORTRAIT: &str = "character_portrait";
pub const PROJECT_FAMILY_CHARACTER_PORTRAIT_IMAGE_PATH_FIELD: &str = "image_path";

#[cfg_attr(test, derive(Default))]
pub struct ProjectFamilyImportPackage {
    pub root_project_id: i64,
    pub projects: Vec<ProjectRecord>,
    pub text_core: ProjectFamilyTextCoreRows,
    pub source_snapshots: Vec<ProjectFamilySourceSnapshotRecord>,
    pub source_snapshot_chapters: Vec<ProjectFamilySourceSnapshotChapterRecord>,
    pub source_snapshot_outlines: Vec<ProjectFamilySourceSnapshotOutlineRecord>,
    pub source_snapshot_profiles: Vec<ProjectFamilySourceSnapshotProfileRecord>,
    pub text_anchors: Vec<ProjectFamilyTextAnchorRecord>,
    pub profile_evidence: Vec<ProjectFamilyProfileEvidenceRecord>,
    pub chapter_visual_plans: Vec<ProjectFamilyVisualPlanRecord>,
    pub chapter_shooting_scripts: Vec<ProjectFamilyShootingScriptRecord>,
    pub authoring_documents: Vec<ProjectFamilyAuthoringDocumentRecord>,
    pub authoring_receipts: Vec<ProjectFamilyAuthoringMigrationReceiptRecord>,
    pub character_portraits: Vec<ProjectFamilyCharacterPortraitRecord>,
    pub generated_images: Vec<ProjectFamilyGeneratedImageRecord>,
    pub chapter_manga_prompts: Vec<ProjectFamilyMangaPromptRecord>,
    pub chapter_video_prompts: Vec<ChapterVideoPromptRecord>,
    pub generated_videos: Vec<ProjectFamilyGeneratedVideoRecord>,
    pub generation_specs: Vec<ProjectFamilyGenerationSpecRecord>,
    pub generation_spec_text_anchors: Vec<ProjectFamilyGenerationSpecTextAnchorRecord>,
    pub generation_spec_entity_refs: Vec<ProjectFamilyGenerationSpecEntityRefRecord>,
    pub generation_spec_media_refs: Vec<ProjectFamilyGenerationSpecMediaRefRecord>,
    pub prompt_artifacts: Vec<ProjectFamilyPromptArtifactRecord>,
    pub media_assets: Vec<ProjectFamilyMediaAssetRecord>,
    pub media_provenance: Vec<ProjectFamilyMediaProvenanceRecord>,
    pub media_bindings: Vec<ProjectFamilyMediaBindingRecord>,
    pub galgame_scripts: Vec<ProjectFamilyGalgameScriptRecord>,
    pub galgame_scenes: Vec<ProjectFamilyGalgameSceneRecord>,
    pub galgame_lines: Vec<ProjectFamilyGalgameLineRecord>,
    pub galgame_choices: Vec<ProjectFamilyGalgameChoiceRecord>,
    pub video_segments: Vec<ProjectFamilyVideoSegmentRecord>,
    pub assembly_timelines: Vec<ProjectFamilyAssemblyTimelineRecord>,
    pub assembly_clips: Vec<ProjectFamilyAssemblyClipRecord>,
    pub source_sync_runs: Vec<ProjectFamilySourceSyncRunRecord>,
    pub source_sync_conflicts: Vec<ProjectFamilySourceSyncConflictRecord>,
    pub source_sync_decisions: Vec<ProjectFamilySourceSyncDecisionRecord>,
}

#[cfg_attr(test, derive(Default))]
pub(super) struct DatabaseProjectFamilyImportPackage {
    pub root_project_id: i64,
    pub projects: Vec<ProjectRecord>,
    pub text_core: DatabaseProjectFamilyTextCoreRows,
    pub source_snapshots: Vec<project_source_snapshot::Model>,
    pub source_snapshot_chapters: Vec<source_snapshot_chapter::Model>,
    pub source_snapshot_outlines: Vec<source_snapshot_outline::Model>,
    pub source_snapshot_profiles: Vec<source_snapshot_profile::Model>,
    pub text_anchors: Vec<text_anchor::Model>,
    pub profile_evidence: Vec<profile_evidence::Model>,
    pub chapter_visual_plans: Vec<chapter_visual_plan::Model>,
    pub chapter_shooting_scripts: Vec<chapter_shooting_script::Model>,
    pub authoring_documents: Vec<a4n_db_models::models::project::media_authoring_document::Model>,
    pub authoring_receipts:
        Vec<a4n_db_models::models::project::media_authoring_migration_receipt::Model>,
    pub character_portraits: Vec<character_portrait::Model>,
    pub generated_images: Vec<generated_image::Model>,
    pub chapter_manga_prompts: Vec<chapter_manga_prompt::Model>,
    pub chapter_video_prompts: Vec<ChapterVideoPromptRecord>,
    pub generated_videos: Vec<generated_video::Model>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(Default))]
pub(super) struct DatabaseProjectFamilyTextCoreRows {
    #[serde(default)]
    pub fact_feedback: Option<crate::fact_feedback::FactFeedbackArchive>,
    pub blueprint: Option<novel_blueprint::Model>,
    pub blueprint_versions: Vec<blueprint_version::Model>,
    pub blueprint_characters: Vec<blueprint_character::Model>,
    pub blueprint_relationships: Vec<blueprint_relationship::Model>,
    pub part_outlines: Vec<part_outline::Model>,
    pub chapter_outlines: Vec<chapter_outline::Model>,
    pub chapters: Vec<chapter::Model>,
    pub character_profiles: Vec<protagonist_profile::Model>,
    pub object_profiles: Vec<object_profile::Model>,
    pub scene_profiles: Vec<scene_profile::Model>,
    pub scene_occurrences: Vec<scene_occurrence::Model>,
    pub scene_transitions: Vec<scene_transition::Model>,
    pub scene_entity_states: Vec<scene_entity_state::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectFamilyImportMappings {
    pub chapter_ids: HashMap<i64, i64>,
    pub source_snapshot_ids: HashMap<i64, i64>,
    pub source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub entity_ids: HashMap<String, HashMap<i64, i64>>,
}

#[derive(Default)]
pub(super) struct ImportIdMaps {
    pub(super) project_ids: HashMap<i64, i64>,
    pub(super) blueprint_ids: HashMap<i64, i64>,
    pub(super) blueprint_version_ids: HashMap<i64, i64>,
    pub(super) part_outline_ids: HashMap<i64, i64>,
    pub(super) chapter_outline_ids: HashMap<i64, i64>,
    pub(super) source_part_outline_ids: HashMap<i64, i64>,
    pub(super) source_chapter_outline_ids: HashMap<i64, i64>,
    pub(super) chapter_ids: HashMap<i64, i64>,
    pub(super) character_profile_ids: HashMap<i64, i64>,
    pub(super) foreshadowing_ids: HashMap<i64, i64>,
    pub(super) object_profile_ids: HashMap<i64, i64>,
    pub(super) scene_profile_ids: HashMap<i64, i64>,
    pub(super) scene_occurrence_ids: HashMap<i64, i64>,
    pub(super) source_snapshot_ids: HashMap<i64, i64>,
    pub(super) source_snapshot_version_hashes: HashMap<i64, String>,
    pub(super) source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub(super) text_anchor_ids: HashMap<i64, i64>,
    pub(super) visual_plan_ids: HashMap<i64, i64>,
    pub(super) visual_plan_sha256s: HashMap<i64, String>,
    pub(super) shooting_script_ids: HashMap<i64, i64>,
    pub(super) shooting_script_sources: HashMap<i64, ShootingScriptImportSource>,
    pub(super) authoring_document_ids: HashMap<i64, i64>,
    pub(super) authoring_document_sha256s: HashMap<i64, String>,
    pub(super) shooting_script_revision_keys: HashMap<i64, String>,
    pub(super) shooting_script_sha256s: HashMap<i64, String>,
    pub(super) shooting_script_shot_fingerprints: HashMap<i64, HashMap<String, (String, String)>>,
    pub(super) character_portrait_ids: HashMap<i64, i64>,
    pub(super) character_portrait_paths: HashMap<i64, String>,
    pub(super) generated_image_ids: HashMap<i64, i64>,
    pub(super) generated_image_paths: HashMap<i64, String>,
    pub(super) chapter_manga_prompt_ids: HashMap<i64, i64>,
    pub(super) chapter_video_prompt_ids: HashMap<i64, i64>,
    pub(super) video_segment_fingerprints: HashMap<String, String>,
    pub(super) video_recipe_fingerprints: HashMap<String, String>,
    pub(super) generated_video_ids: HashMap<i64, i64>,
    pub(super) generated_video_paths: HashMap<i64, String>,
    pub(super) generated_video_thumbnail_paths: HashMap<i64, String>,
    pub(super) generation_spec_ids: HashMap<i64, i64>,
    pub(super) prompt_artifact_ids: HashMap<i64, i64>,
    pub(super) media_asset_ids: HashMap<i64, i64>,
    pub(super) galgame_script_ids: HashMap<i64, i64>,
    pub(super) galgame_scene_ids: HashMap<i64, i64>,
    pub(super) galgame_line_ids: HashMap<i64, i64>,
    pub(super) video_segment_ids: HashMap<i64, i64>,
    pub(super) assembly_timeline_ids: HashMap<i64, i64>,
    pub(super) assembly_clip_ids: HashMap<i64, i64>,
    pub(super) source_sync_run_ids: HashMap<i64, i64>,
    pub(super) source_sync_conflict_ids: HashMap<i64, i64>,
}

pub(super) struct ShootingScriptImportSource {
    pub project_id: i64,
    pub revision: i32,
    pub sha256: String,
    pub source_revision_key: String,
}

impl From<&chapter_shooting_script::Model> for ShootingScriptImportSource {
    fn from(script: &chapter_shooting_script::Model) -> Self {
        Self {
            project_id: script.project_id,
            revision: script.revision,
            sha256: script.script_sha256.clone(),
            source_revision_key: script.source_revision_key.clone(),
        }
    }
}

pub trait ProjectFamilyMediaFileImporter {
    fn write_imported_media_field(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        old_owner_id: i64,
        new_owner_id: i64,
        field: &str,
    ) -> Result<Option<String>, super::ProjectFamilyMediaImportError>;
}
