use super::super::cleanup::{
    ProjectCleanupDbError, ProjectGeneratedArtifactLinkCleanupResult,
    ProjectGeneratedArtifactRowCleanupResult, ProjectTaskQueueCleanupResult,
};
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeformGeneratedResetTargets {
    pub image_ids: Vec<i64>,
    pub video_ids: Vec<i64>,
    pub media_asset_ids: Vec<i64>,
    pub generation_spec_ids: Vec<i64>,
    pub prompt_artifact_ids: Vec<i64>,
    pub media_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeformVideoGeneratedDataResetResult {
    pub task_cleanup: ProjectTaskQueueCleanupResult,
    pub artifact_link_cleanup: ProjectGeneratedArtifactLinkCleanupResult,
    pub artifact_row_cleanup: ProjectGeneratedArtifactRowCleanupResult,
    pub media_paths: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum FreeformGeneratedResetDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目不存在: {project_id}")]
    ProjectNotFound { project_id: i64 },
    #[error("仅自由画布视频项目支持该生成数据重置入口")]
    UnsupportedProject,
    #[error("{source}")]
    ProjectCleanup {
        #[source]
        source: ProjectCleanupDbError,
    },
    #[error("{source}")]
    Canvas {
        #[source]
        source: crate::canvas::CanvasDbError,
    },
    #[error("{source}")]
    Database {
        #[source]
        source: DbErr,
    },
}

impl From<ProjectCleanupDbError> for FreeformGeneratedResetDbError {
    fn from(source: ProjectCleanupDbError) -> Self {
        Self::ProjectCleanup { source }
    }
}

impl From<crate::canvas::CanvasDbError> for FreeformGeneratedResetDbError {
    fn from(source: crate::canvas::CanvasDbError) -> Self {
        Self::Canvas { source }
    }
}

impl From<DbErr> for FreeformGeneratedResetDbError {
    fn from(source: DbErr) -> Self {
        Self::Database { source }
    }
}
