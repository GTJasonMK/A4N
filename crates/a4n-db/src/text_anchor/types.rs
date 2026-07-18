use a4n_core::models::{chapter::chapter, project::text_anchor};
use sea_orm::DbErr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAnchorProfileKind {
    Character,
    Object,
    Scene,
}

impl TextAnchorProfileKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Object => "object",
            Self::Scene => "scene",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextAnchorEvidenceInput {
    pub text_anchor_id: i64,
    pub evidence_role: String,
}

#[derive(Debug, Clone)]
pub struct CurrentChapterContentAnchorRows {
    pub chapter: chapter::Model,
    pub anchors: Vec<text_anchor::Model>,
    pub source_version_hash: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TextAnchorDbError {
    #[error("数据库错误: {0}")]
    Database(#[from] DbErr),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
}

pub(super) type TextAnchorDbResult<T> = Result<T, TextAnchorDbError>;

#[derive(Debug, Clone)]
pub struct TextAnchorContentInsertInput<'a> {
    pub project_id: i64,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_id: i64,
    pub chapter_number: i32,
    pub content: &'a str,
    pub source_version_hash: &'a str,
}
