use a4n_core::models::project::{novel_project, project_source_snapshot};
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ProjectFamilyContext {
    pub project: novel_project::Model,
    pub root_project: novel_project::Model,
    pub source_snapshot: project_source_snapshot::Model,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectFamilyDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MangaCharacterPortraitRefSummary {
    pub profile_id: i64,
    pub portrait_id: i64,
    pub character_name: String,
    pub image_path: String,
    pub prompt: String,
    pub portrait_type: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChapterMangaSourceSummary {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub status: String,
    pub content: String,
}
