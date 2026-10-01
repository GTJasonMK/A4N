use a4n_domain::project::ProjectSummaryRecord;
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct InspirationProjectContextRows {
    pub project: ProjectSummaryRecord,
    pub blueprint: Option<Value>,
    pub recent_chapters: Vec<InspirationRecentChapterRow>,
}

#[derive(Debug, Clone)]
pub struct InspirationRecentChapterRow {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub analysis_data: Option<Value>,
    pub word_count: i32,
    pub has_content: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum InspirationDbError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
