use a4n_db_models::models::chapter::{chapter, chapter_outline};
use a4n_domain::{chapter::ChapterSourceSnapshot, source_text_hash};
use chrono::NaiveDateTime;
use sea_orm::DbErr;
use serde_json::Value;

pub fn chapter_source_snapshot_from_model(chapter: &chapter::Model) -> ChapterSourceSnapshot {
    ChapterSourceSnapshot {
        project_id: chapter.project_id,
        chapter_id: chapter.id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        content_sha256: source_text_hash(&chapter.content),
        updated_at: chapter.updated_at,
    }
}

pub(crate) fn chapter_source_snapshot_matches(
    source: &ChapterSourceSnapshot,
    chapter: &chapter::Model,
) -> bool {
    chapter_source_snapshot_matches_fields(
        source,
        chapter.id,
        chapter.project_id,
        chapter.chapter_number,
        &chapter.title,
        &chapter.content,
        chapter.updated_at,
    )
}

pub(crate) fn chapter_source_snapshot_matches_fields(
    source: &ChapterSourceSnapshot,
    chapter_id: i64,
    project_id: i64,
    chapter_number: i32,
    title: &Option<String>,
    content: &str,
    updated_at: NaiveDateTime,
) -> bool {
    chapter_source_snapshot_targets(source, project_id, chapter_number)
        && source.chapter_id == chapter_id
        && source.title == *title
        && source.content_sha256 == source_text_hash(content)
        && source.updated_at == updated_at
}

pub(crate) fn chapter_source_snapshot_targets(
    source: &ChapterSourceSnapshot,
    project_id: i64,
    chapter_number: i32,
) -> bool {
    source.project_id == project_id && source.chapter_number == chapter_number
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineGenerationProgressPart {
    pub id: i64,
    pub start_chapter: i32,
    pub end_chapter: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineGenerationProgressChapter {
    pub id: i64,
    pub chapter_number: i32,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct OutlineGenerationProgressRows {
    pub parts: Vec<OutlineGenerationProgressPart>,
    pub chapter_outlines: Vec<OutlineGenerationProgressChapter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectChapterOutlineUpdateInput {
    pub project_id: i64,
    pub outline_id: i64,
    pub expected_updated_at: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualChapterOutlineSaveInput {
    pub project_id: i64,
    pub outline_id: Option<i64>,
    pub expected_updated_at: Option<String>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualPartOutlineSaveInput {
    pub project_id: i64,
    pub part_id: Option<i64>,
    pub expected_updated_at: Option<String>,
    pub part_number: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub theme: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineDeleteTarget {
    pub id: i64,
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteLatestOutlineRecordsInput {
    pub project_id: i64,
    pub targets: Vec<OutlineDeleteTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrospectiveChapterOutlineUpsertInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub title: String,
    pub summary: String,
    pub key_scenes: Option<Value>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: i32,
    pub source_fingerprint_json: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrospectiveChapterOutlineAction {
    Created,
    Updated,
    Unchanged,
    Preserved,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetrospectiveChapterOutlineUpsertOutcome {
    pub outline: chapter_outline::Model,
    pub action: RetrospectiveChapterOutlineAction,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectChapterOutlineUpdateError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("项目状态 {current} 不允许执行：{operation}")]
    InvalidState { current: String, operation: String },
    #[error("保存旧章节大纲表单；章节大纲已经变化，请刷新后重新编辑")]
    SourceChanged,
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectOutlineRecordError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目状态 {current} 不允许执行：{operation}")]
    InvalidState { current: String, operation: String },
    #[error("章节正文已变化，拒绝写入旧分析生成的章纲: chapter_id={chapter_id}")]
    ChapterSourceChanged { chapter_id: i64 },
    #[error("{operation}；大纲资料已经变化，请刷新后重新编辑")]
    OutlineSourceChanged { operation: String },
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl From<crate::outline::OutlineDbError> for ProjectOutlineRecordError {
    fn from(error: crate::outline::OutlineDbError) -> Self {
        match error {
            crate::outline::OutlineDbError::Database(error) => Self::Database(error),
            crate::outline::OutlineDbError::InvalidArgument(message) => {
                Self::InvalidArgument(message)
            }
            crate::outline::OutlineDbError::InvalidState { current, operation } => {
                Self::InvalidState { current, operation }
            }
            crate::outline::OutlineDbError::Generation(error) => {
                Self::InvalidArgument(error.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectOutlineRecordError;
    use crate::{generation::GenerationDbError, outline::OutlineDbError};

    #[test]
    fn outline_db_errors_preserve_project_outline_record_mapping() {
        let converted: ProjectOutlineRecordError =
            OutlineDbError::InvalidArgument("invalid outline".into()).into();
        assert!(matches!(
            converted,
            ProjectOutlineRecordError::InvalidArgument(message)
                if message == "invalid outline"
        ));

        let converted: ProjectOutlineRecordError = OutlineDbError::InvalidState {
            current: "stale outline".into(),
            operation: "save outline".into(),
        }
        .into();
        assert!(matches!(
            converted,
            ProjectOutlineRecordError::InvalidState { current, operation }
                if current == "stale outline" && operation == "save outline"
        ));

        let generation_error = GenerationDbError::InvalidArgument("bad generation input".into());
        let converted: ProjectOutlineRecordError =
            OutlineDbError::Generation(generation_error).into();
        assert!(matches!(
            converted,
            ProjectOutlineRecordError::InvalidArgument(message)
                if message == "bad generation input"
        ));

        let database_error = sea_orm::DbErr::Custom("database unavailable".into());
        let database_message = database_error.to_string();
        let converted: ProjectOutlineRecordError = OutlineDbError::Database(database_error).into();
        assert!(matches!(
            converted,
            ProjectOutlineRecordError::Database(error)
                if error.to_string() == database_message
        ));
    }
}
