use sea_orm::DbErr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRollbackImpactQuery {
    pub project_id: i64,
    pub delete_chapters: bool,
    pub delete_chapter_outlines: bool,
    pub delete_part_outlines: bool,
    pub delete_blueprint_versions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRollbackImpactSummary {
    pub kind: String,
    pub title: String,
    pub tables: Vec<String>,
    pub count: Option<i32>,
    pub chapter_start: Option<i32>,
    pub chapter_end: Option<i32>,
    pub part_start: Option<i32>,
    pub part_end: Option<i32>,
    pub note: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectRollbackImpactReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectRollbackImpactReadError {
    pub(in crate::project::rollback_impact) fn database(
        operation: &'static str,
        source: DbErr,
    ) -> Self {
        Self::Database { operation, source }
    }
}
