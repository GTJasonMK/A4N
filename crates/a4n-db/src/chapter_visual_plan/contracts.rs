use a4n_db_models::models::chapter::chapter_visual_plan;
use a4n_domain::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, ChapterVisualPlanStatus, ChapterVisualPlanV2,
};
use sea_orm::DbErr;

use crate::project::ResolvedChapterSource;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChapterVisualPlan {
    pub record: chapter_visual_plan::Model,
    pub status: ChapterVisualPlanStatus,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub plan: ChapterVisualPlanV2,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChapterVisualPlanSet {
    pub draft: Option<ParsedChapterVisualPlan>,
    pub confirmed: Option<ParsedChapterVisualPlan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedVisualPlanDraft {
    pub record_id: i64,
    pub revision: i32,
}

#[derive(Debug, Clone)]
pub struct SaveChapterVisualPlanDraftInput {
    pub source: ResolvedChapterSource,
    pub author_user_id: i64,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub expected: Option<ExpectedVisualPlanDraft>,
    pub expected_source_hash: Option<String>,
    pub plan: ChapterVisualPlanV2,
}

#[derive(Debug, Clone)]
pub struct ConfirmChapterVisualPlanInput {
    pub record_id: i64,
    pub expected_revision: i32,
}

#[derive(Debug, Clone)]
pub struct ConfirmChapterVisualPlanOutcome {
    pub visual_plan: ParsedChapterVisualPlan,
    pub invalidated_project_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct DiscardChapterVisualPlanDraftInput {
    pub record_id: i64,
    pub expected_revision: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum ChapterVisualPlanDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("{0}")]
    NotFound(String),
    #[error("章节视觉编排 revision 冲突: id={record_id}, expected_revision={expected_revision}")]
    RevisionConflict {
        record_id: i64,
        expected_revision: i32,
    },
    #[error(transparent)]
    Database(#[from] DbErr),
}
