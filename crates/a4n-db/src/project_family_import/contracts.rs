use std::collections::HashMap;

use crate::project::ProjectFamilyTextCoreRows;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{
        chapter_manga_prompt, chapter_shooting_script, chapter_video_prompt, chapter_visual_plan,
    },
    character::character_portrait,
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

pub const PROJECT_FAMILY_MEDIA_OWNER_CHARACTER_PORTRAIT: &str = "character_portrait";
pub const PROJECT_FAMILY_CHARACTER_PORTRAIT_IMAGE_PATH_FIELD: &str = "image_path";

#[cfg_attr(test, derive(Default))]
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
    pub chapter_visual_plans: Vec<chapter_visual_plan::Model>,
    pub chapter_shooting_scripts: Vec<chapter_shooting_script::Model>,
    pub character_portraits: Vec<character_portrait::Model>,
    pub generated_images: Vec<generated_image::Model>,
    pub chapter_manga_prompts: Vec<chapter_manga_prompt::Model>,
    pub chapter_video_prompts: Vec<chapter_video_prompt::Model>,
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

#[derive(Default)]
pub(in crate::project_family_import) struct ImportIdMaps {
    pub(in crate::project_family_import) project_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) blueprint_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) blueprint_version_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) part_outline_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) chapter_outline_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) chapter_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) character_profile_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) object_profile_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) scene_profile_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) scene_occurrence_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) source_snapshot_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) source_snapshot_version_hashes: HashMap<i64, String>,
    pub(in crate::project_family_import) source_snapshot_chapter_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) text_anchor_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) visual_plan_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) visual_plan_sha256s: HashMap<i64, String>,
    pub(in crate::project_family_import) shooting_script_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) shooting_script_revision_keys: HashMap<i64, String>,
    pub(in crate::project_family_import) shooting_script_sha256s: HashMap<i64, String>,
    pub(in crate::project_family_import) character_portrait_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) character_portrait_paths: HashMap<i64, String>,
    pub(in crate::project_family_import) generated_image_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) generated_image_paths: HashMap<i64, String>,
    pub(in crate::project_family_import) chapter_manga_prompt_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) chapter_video_prompt_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) generated_video_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) generated_video_paths: HashMap<i64, String>,
    pub(in crate::project_family_import) generated_video_thumbnail_paths: HashMap<i64, String>,
    pub(in crate::project_family_import) generation_spec_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) prompt_artifact_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) media_asset_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) galgame_script_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) galgame_scene_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) galgame_line_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) video_segment_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) assembly_timeline_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) source_sync_run_ids: HashMap<i64, i64>,
    pub(in crate::project_family_import) source_sync_conflict_ids: HashMap<i64, i64>,
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
