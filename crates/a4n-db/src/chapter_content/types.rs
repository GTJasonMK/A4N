use a4n_core::models::chapter::chapter_outline;
use chrono::NaiveDateTime;
use serde_json::Value;

use crate::generation::GenerationDbError;

#[derive(Debug, Clone)]
pub struct ImportChapterContentInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EditChapterContentInput {
    pub chapter_id: i64,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChapterGenerationExistingRecord {
    pub id: i64,
    pub previous_status: String,
    pub title: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratedChapterContentRecordInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub existing_chapter_id: Option<i64>,
    pub source_outline: Option<chapter_outline::Model>,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub word_count: i32,
    pub now: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct StoreChapterAnalysisRecordInput {
    pub analysis_data: Value,
    pub summary: Option<String>,
    pub style_tags: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ChapterContentImportError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
    #[error("第 {chapter_number} 章正在生成中，请等待当前生成完成后再{operation}")]
    TargetChapterGenerating {
        chapter_number: i32,
        status: String,
        operation: String,
    },
}

impl ChapterContentImportError {
    pub(super) fn database(error: sea_orm::DbErr) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChapterContentEditError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("当前状态 {current} 不允许执行 {operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
}

impl ChapterContentEditError {
    pub(super) fn database(error: sea_orm::DbErr) -> Self {
        Self::Database(error)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChapterGenerationLifecycleError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("当前状态 {current} 不允许执行 {operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
}

impl ChapterGenerationLifecycleError {
    pub(super) fn database(error: sea_orm::DbErr) -> Self {
        Self::Database(error)
    }
}

impl From<GenerationDbError> for ChapterGenerationLifecycleError {
    fn from(error: GenerationDbError) -> Self {
        match error {
            GenerationDbError::InvalidArgument(message) => Self::InvalidArgument(message),
            GenerationDbError::NotFound(message) => Self::NotFound(message),
            GenerationDbError::Database(error) => Self::Database(error),
        }
    }
}
