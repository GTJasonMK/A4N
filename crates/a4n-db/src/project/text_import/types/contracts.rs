use super::payload::ImportTextProjectPayload;
use a4n_core::models::project::novel_project;
use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectRecordInput {
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub status: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,
    pub raw_blueprint: Option<String>,
    pub import_analysis_progress: Option<String>,
    pub export_config: Option<String>,
    pub payload: ImportTextProjectPayload,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportTextProjectRecordResult {
    pub project: novel_project::Model,
    pub postprocess_chapter_ids: Vec<i64>,
    pub has_preserved_chapter_tags: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportTextProjectError {
    #[error("{0}")]
    InvalidPackage(String),
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ImportTextProjectError {
    pub(in crate::project::text_import) fn database(
        operation: &'static str,
        source: DbErr,
    ) -> Self {
        Self::Database { operation, source }
    }
}
