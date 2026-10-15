use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::collaboration::ProjectAccessRole;
pub use a4n_domain::collaboration::ProjectAccessScope;
pub use a4n_domain::project::{
    AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode, AssetGraphNodeDetail,
    AssetLineageRecord, AssetLineageRef, GenerationInputSource, ProjectKind,
    ProjectProgressSummary, ProjectRollbackImpactSummary, ProjectRollbackPreview,
    ProjectSourceMode, ProjectStatus, ProjectStatusHistorySummary, ProjectStatusMachineSummary,
    PromptSource, SourceSnapshotChapterIndex, SourceSnapshotChapterSummary,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSummary {
    pub id: i64,
    pub content_revision: i64,
    pub fact_feedback_interval_chapters: i32,
    pub title: String,
    pub description: String,
    pub lifecycle_gate: String,
    pub project_type: String,
    pub project_kind: ProjectKind,
    pub source_mode: ProjectSourceMode,
    pub root_project_id: Option<i64>,
    pub source_sync_status: String,
    pub root_project_title: Option<String>,
    pub derived_chapter_scope: Option<Value>,
    pub status: ProjectStatus,
    pub current_chapter: i32,
    pub target_chapters: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub owner_username: String,
    pub my_role: ProjectAccessRole,
    pub collaboration_root_project_id: i64,
    pub member_count: u64,
    pub pending_invite_count: Option<u64>,
    pub progress: ProjectProgressSummary,
    pub updated_at: String,
}

/// Host adapter for project duplication.
///
/// The project owner keeps permission checks and database transactions; the
/// host only supplies media copying and rollback side effects.
#[async_trait::async_trait]
pub trait ProjectDuplicationHostPort:
    a4n_domain::project::ProjectMediaFileCopier + Send + Sync
{
    async fn verify_project_editor(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn rollback_copied_media_files(&mut self, label: &str) -> Result<(), String>;
}

/// Host-facing project duplication boundary.
#[async_trait::async_trait]
pub trait ProjectDuplicationApplicationPort: Send + Sync {
    async fn duplicate_project(
        &self,
        user_id: i64,
        input: DuplicateProjectInput,
        host_port: &mut dyn ProjectDuplicationHostPort,
    ) -> Result<ProjectSummary, a4n_error::AppError>;
}

/// Host-facing project status read boundary owned by the project-status worker.
#[async_trait::async_trait]
pub trait ProjectStatusReadApplicationPort: Send + Sync {
    async fn status_machine(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectStatusMachineSummary, a4n_error::AppError>;

    async fn preview_rollback(
        &self,
        user_id: i64,
        project_id: i64,
        expected_from: ProjectStatus,
        target: ProjectStatus,
    ) -> Result<ProjectRollbackPreview, a4n_error::AppError>;
}

/// Host-facing project status mutation boundary.
#[async_trait::async_trait]
pub trait ProjectStatusMutationApplicationPort: Send + Sync {
    async fn forward_status(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectStatusMutationResult, a4n_error::AppError>;

    async fn rollback_status(
        &self,
        user_id: i64,
        project_id: i64,
        expected_from: ProjectStatus,
        target: ProjectStatus,
    ) -> Result<ProjectStatusMutationResult, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectStatusMutationResult {
    pub project_id: i64,
    pub from: ProjectStatus,
    pub to: ProjectStatus,
    #[schemars(with = "String")]
    pub changed_at: NaiveDateTime,
    pub needs_rag_clear: bool,
    pub post_commit_errors: Vec<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PaginatedProjects {
    pub items: Vec<ProjectSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectListInput {
    pub page: u64,
    pub page_size: u64,
    pub project_kind: Option<ProjectKind>,
    pub root_project_id: Option<i64>,
    pub access_scope: Option<ProjectAccessScope>,
}

/// Host-facing project read boundary.
///
/// Project owner code returns complete API summaries, so Runtime does not
/// import project ORM models or duplicate family/progress/access projections.
#[async_trait::async_trait]
pub trait ProjectReadApplicationPort: Send + Sync {
    async fn summary(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectSummary, a4n_error::AppError>;

    async fn list(
        &self,
        user_id: i64,
        input: ProjectListInput,
    ) -> Result<PaginatedProjects, a4n_error::AppError>;

    async fn all_project_ids(&self) -> Result<Vec<i64>, a4n_error::AppError>;

    async fn exists(&self, project_id: i64) -> Result<bool, a4n_error::AppError>;
}

/// Host-facing source snapshot chapter read boundary.
#[async_trait::async_trait]
pub trait ProjectSourceSnapshotReadApplicationPort: Send + Sync {
    async fn chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, a4n_error::AppError>;

    async fn chapter_index(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterIndex>, a4n_error::AppError>;

    async fn chapter(
        &self,
        user_id: i64,
        project_id: i64,
        snapshot_id: Option<i64>,
        source_chapter_id: i64,
    ) -> Result<SourceSnapshotChapterSummary, a4n_error::AppError>;
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetSourceSnapshotChapterInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    pub snapshot_id: Option<i64>,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
}

/// Host-facing project mutation boundary.
///
/// Validation, permission checks and the final summary projection stay in the
/// project owner; Runtime only routes the typed command.
#[async_trait::async_trait]
pub trait ProjectMutationApplicationPort: Send + Sync {
    async fn create(
        &self,
        user_id: i64,
        input: CreateProjectInput,
    ) -> Result<ProjectSummary, a4n_error::AppError>;

    async fn create_derived(
        &self,
        user_id: i64,
        input: CreateDerivedProjectInput,
    ) -> Result<ProjectSummary, a4n_error::AppError>;

    async fn create_canvas(
        &self,
        user_id: i64,
        input: CreateCanvasProjectInput,
    ) -> Result<ProjectSummary, a4n_error::AppError>;

    async fn create_standalone_comic(
        &self,
        user_id: i64,
        input: CreateStandaloneComicProjectInput,
    ) -> Result<ProjectSummary, a4n_error::AppError>;

    async fn update(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateProjectInput,
    ) -> Result<ProjectSummary, a4n_error::AppError>;
}

/// Host-facing project asset dependency graph boundary.
#[async_trait::async_trait]
pub trait ProjectAssetGraphApplicationPort: Send + Sync {
    async fn graph(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<AssetDependencyGraph, a4n_error::AppError>;

    async fn node_detail(
        &self,
        user_id: i64,
        project_id: i64,
        node_id: String,
    ) -> Result<AssetGraphNodeDetail, a4n_error::AppError>;
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateProjectInput {
    pub title: String,
    pub description: Option<String>,
    pub project_type: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCanvasProjectInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateStandaloneComicProjectInput {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateDerivedProjectInput {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DuplicateProjectInput {
    pub project_id: i64,
    pub new_title: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProjectInput {
    pub title: Option<String>,
    pub description: Option<String>,
    #[schemars(range(min = 1))]
    pub fact_feedback_interval_chapters: Option<i32>,
    #[schemars(with = "Option<String>")]
    pub expected_updated_at: Option<NaiveDateTime>,
}

#[derive(Serialize, JsonSchema)]
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

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectGeneratedDataResetStatus {
    Completed,
    Pending,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectGeneratedDataResetResponse {
    pub status: ProjectGeneratedDataResetStatus,
    pub database_committed: bool,
    pub result: Option<ProjectGeneratedDataResetSummary>,
    pub warnings: Vec<String>,
}
