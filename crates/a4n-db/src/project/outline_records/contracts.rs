use a4n_core::models::chapter::{chapter_outline, part_outline};
use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct OutlineGenerationProgressRows {
    pub parts: Vec<part_outline::Model>,
    pub chapter_outlines: Vec<chapter_outline::Model>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectChapterOutlineUpdateInput {
    pub project_id: i64,
    pub outline_id: i64,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectChapterOutlineUpdateError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectOutlineRecordError {
    #[error(transparent)]
    Database(#[from] DbErr),
}
