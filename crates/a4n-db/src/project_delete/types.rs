use super::*;

#[derive(Debug, thiserror::Error)]
pub enum ProjectDeleteDbError {
    #[error("项目不存在: project_id={project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("文本项目存在 {derived_count} 个派生项目，请先删除派生项目或执行项目族删除")]
    DerivedProjectsExist { project_id: i64, derived_count: u64 },
    #[error("{source}")]
    ProjectCleanup {
        #[source]
        source: project_db::ProjectCleanupDbError,
    },
    #[error("{source}")]
    Generation {
        #[source]
        source: generation::GenerationDbError,
    },
    #[error("{source}")]
    Video {
        #[source]
        source: crate::video::VideoDbError,
    },
    #[error("{source}")]
    Database {
        #[source]
        source: DbErr,
    },
}

impl From<DbErr> for ProjectDeleteDbError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}

impl From<generation::GenerationDbError> for ProjectDeleteDbError {
    fn from(source: generation::GenerationDbError) -> Self {
        Self::Generation { source }
    }
}

impl From<project_db::ProjectCleanupDbError> for ProjectDeleteDbError {
    fn from(source: project_db::ProjectCleanupDbError) -> Self {
        Self::ProjectCleanup { source }
    }
}

impl From<crate::video::VideoDbError> for ProjectDeleteDbError {
    fn from(source: crate::video::VideoDbError) -> Self {
        Self::Video { source }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GeneratedImageDeleteScope {
    Project,
    Chapters,
}

#[derive(Debug, Clone, Default)]
pub struct ChapterGenerationCleanupResult {
    pub deleted_manga_prompts: u64,
    pub deleted_video_prompts: u64,
    pub deleted_chapter_scenes: u64,
    pub deleted_generated_images: u64,
    pub deleted_generated_videos: u64,
    pub candidate_media_paths: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProfileSceneCleanupResult {
    pub deleted_object_profiles: u64,
    pub deleted_scene_profiles: u64,
    pub deleted_scene_occurrences: u64,
    pub deleted_scene_transitions: u64,
    pub deleted_scene_entity_states: u64,
}
