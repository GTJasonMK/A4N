use serde::{Deserialize, Serialize};

pub use a4n_orchestrator::project::{
    AssetDependencyGraph, AssetGraphNodeDetail, ProjectProgressStatus, ProjectProgressSummary,
    SourceSnapshotChapterSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub project_kind: String,
    pub source_mode: String,
    pub root_project_id: Option<i64>,
    pub parent_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_sync_status: String,
    pub derived_project_status: Option<String>,
    pub root_project_title: Option<String>,
    pub source_snapshot_created_at: Option<String>,
    pub derived_chapter_scope: Option<serde_json::Value>,
    pub status: String,
    pub current_chapter: i32,
    pub target_chapters: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub owner_user_id: i64,
    pub owner_username: Option<String>,
    pub my_role: Option<String>,
    pub collaboration_root_project_id: i64,
    pub member_count: u64,
    pub pending_invite_count: Option<u64>,
    pub progress: ProjectProgressSummary,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedProjects {
    pub items: Vec<ProjectSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectInput {
    pub title: String,
    pub description: Option<String>,
    pub project_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateFreeformVideoProjectInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateDerivedProjectInput {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: Option<String>,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub creation_options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectInput {
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGeneratedDataResetSummary {
    pub project: ProjectSummary,
    pub deleted_galgame_scripts: u64,
    pub deleted_galgame_scenes: u64,
    pub deleted_galgame_lines: u64,
    pub deleted_galgame_choices: u64,
    pub deleted_chapters: u64,
    pub deleted_chapter_outlines: u64,
    pub deleted_part_outlines: u64,
    pub deleted_chapter_derived_rows: u64,
    pub deleted_current_blueprints: u64,
    pub deleted_blueprint_characters: u64,
    pub deleted_blueprint_relationships: u64,
    pub preserved_blueprint_versions: u64,
    pub reset_character_profiles: u64,
    pub deleted_character_profiles: u64,
    pub deleted_character_snapshots: u64,
    pub deleted_character_timeline_rows: u64,
    pub deleted_character_portraits: u64,
    pub deleted_object_profiles: u64,
    pub deleted_foreshadowing: u64,
    pub deleted_scene_profiles: u64,
    pub deleted_scene_occurrences: u64,
    pub deleted_scene_transitions: u64,
    pub deleted_scene_entity_states: u64,
    pub deleted_text_anchors: u64,
    pub deleted_profile_evidence: u64,
    pub deleted_import_reviews: u64,
    pub deleted_conversations: u64,
    pub deleted_conversation_messages: u64,
    pub deleted_optimization_sessions: u64,
    pub deleted_optimization_session_messages: u64,
    pub deleted_optimization_turns: u64,
    pub deleted_optimization_events: u64,
    pub deleted_optimization_patch_sets: u64,
    pub deleted_optimization_patches: u64,
    pub deleted_generated_images: u64,
    pub deleted_generated_videos: u64,
    pub deleted_generation_specs: u64,
    pub deleted_prompt_artifacts: u64,
    pub deleted_media_assets: u64,
    pub deleted_media_provenance: u64,
    pub deleted_media_bindings: u64,
    pub deleted_tasks: u64,
    pub deleted_task_dependencies: u64,
    pub deleted_media_files: usize,
}
