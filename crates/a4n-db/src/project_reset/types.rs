use super::*;

#[derive(Debug, thiserror::Error)]
pub enum ProjectResetDbError {
    #[error("项目不存在: {project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("重置生成态不支持项目类型: {project_kind}")]
    UnsupportedProjectKind { project_kind: String },
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("项目状态已变化，请刷新后重试: expected={expected}, actual={actual}")]
    StatusChanged { expected: String, actual: String },
    #[error("{0}")]
    Internal(String),
    #[error("{source}")]
    CharacterProfileDelete {
        #[source]
        source: character::CharacterProfileDeleteError,
    },
    #[error("{source}")]
    Generation {
        #[source]
        source: generation::GenerationDbError,
    },
    #[error("{source}")]
    ProjectCleanup {
        #[source]
        source: crate::project_delete::ProjectDeleteDbError,
    },
    #[error("{source}")]
    ProjectRecordCleanup {
        #[source]
        source: project_db::ProjectCleanupDbError,
    },
    #[error("{source}")]
    ProjectStatusWrite {
        #[source]
        source: project_db::ProjectStatusWriteError,
    },
    #[error("{source}")]
    ProjectMutation {
        #[source]
        source: project_db::ProjectMutationError,
    },
    #[error("{source}")]
    Database {
        #[source]
        source: DbErr,
    },
}

impl From<DbErr> for ProjectResetDbError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}

impl From<character::CharacterProfileDeleteError> for ProjectResetDbError {
    fn from(source: character::CharacterProfileDeleteError) -> Self {
        Self::CharacterProfileDelete { source }
    }
}

impl From<generation::GenerationDbError> for ProjectResetDbError {
    fn from(source: generation::GenerationDbError) -> Self {
        Self::Generation { source }
    }
}

impl From<crate::project_delete::ProjectDeleteDbError> for ProjectResetDbError {
    fn from(source: crate::project_delete::ProjectDeleteDbError) -> Self {
        Self::ProjectCleanup { source }
    }
}

impl From<project_db::ProjectCleanupDbError> for ProjectResetDbError {
    fn from(source: project_db::ProjectCleanupDbError) -> Self {
        Self::ProjectRecordCleanup { source }
    }
}

impl From<project_db::ProjectStatusWriteError> for ProjectResetDbError {
    fn from(source: project_db::ProjectStatusWriteError) -> Self {
        Self::ProjectStatusWrite { source }
    }
}

impl From<project_db::ProjectMutationError> for ProjectResetDbError {
    fn from(source: project_db::ProjectMutationError) -> Self {
        Self::ProjectMutation { source }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectGeneratedDataResetResult {
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
}

#[derive(Debug, Clone)]
pub struct ProjectGeneratedDataResetOutcome {
    pub result: ProjectGeneratedDataResetResult,
    pub previous_status: ProjectStatus,
    pub media_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectStatusRollbackOutcome {
    pub project: novel_project::Model,
    pub needs_rag_clear: bool,
}
