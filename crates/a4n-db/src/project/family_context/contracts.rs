use a4n_db_models::models::project::{novel_project, project_source_snapshot};
use sea_orm::{DbErr, FromQueryResult};

#[derive(Debug, Clone, FromQueryResult)]
pub(crate) struct ProjectFamilyAccessRow {
    pub id: i64,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub(crate) struct ProjectFamilySnapshotAccessRow {
    pub id: i64,
    pub root_project_id: i64,
}

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

#[derive(Debug, Clone)]
pub struct MangaCharacterPortraitRefSummary {
    pub profile_id: i64,
    pub portrait_id: i64,
    pub character_name: String,
    pub image_path: String,
    pub prompt: String,
    pub portrait_type: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
}

impl a4n_domain::manga::MangaPortraitReference for MangaCharacterPortraitRefSummary {
    fn profile_id(&self) -> i64 {
        self.profile_id
    }

    fn portrait_id(&self) -> i64 {
        self.portrait_id
    }

    fn character_name(&self) -> &str {
        &self.character_name
    }

    fn image_path(&self) -> &str {
        &self.image_path
    }

    fn prompt(&self) -> &str {
        &self.prompt
    }

    fn portrait_type(&self) -> &str {
        &self.portrait_type
    }

    fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    fn pose(&self) -> Option<&str> {
        self.pose.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct ChapterMangaSourceSummary {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub status: String,
    pub content: String,
}
