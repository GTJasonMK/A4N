use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{
        chapter, chapter_manga_prompt, chapter_outline, chapter_shooting_script,
        chapter_video_prompt, chapter_visual_plan, part_outline,
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
    scene::{scene_entity_state, scene_occurrence, scene_transition},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_domain::{
    object_profile::ObjectProfileRecord, project::ProjectRecord, scene_profile::SceneProfileRecord,
};
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ProjectFamilyExportError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Clone)]
pub struct ProjectFamilyExportSelection {
    pub root: ProjectRecord,
    pub derived_projects: Vec<ProjectRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectFamilyTextCoreRows {
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
    pub object_profiles: Vec<ObjectProfileRecord>,
    pub scene_profiles: Vec<SceneProfileRecord>,
    pub scene_occurrences: Vec<scene_occurrence::Model>,
    pub scene_transitions: Vec<scene_transition::Model>,
    pub scene_entity_states: Vec<scene_entity_state::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectFamilyExportRows {
    pub snapshot_ids: Vec<i64>,
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
