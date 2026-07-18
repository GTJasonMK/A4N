use a4n_core::models::chapter::chapter_scene;
use chrono::NaiveDateTime;
use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectChapterSceneUpdateInput {
    pub scene: chapter_scene::Model,
    pub content: Option<String>,
    pub title: Option<Option<String>>,
    pub summary: Option<Option<String>>,
    pub scene_type: Option<String>,
    pub location: Option<Option<String>>,
    pub timeline_position: Option<Option<String>>,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectChapterSceneRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{operation}失败: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectChapterSceneRecordError {
    pub(super) fn database(operation: &'static str, source: DbErr) -> Self {
        Self::Database { operation, source }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChapterSceneReorderInput {
    pub order: Vec<(i64, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectChapterSceneReorderScope {
    pub chapter_id: i64,
    pub project_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectGeneratedChapterSceneSaveInput {
    pub chapter_id: i64,
    pub scene_number: i32,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectChapterSceneReorderError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectGeneratedChapterSceneSaveError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
