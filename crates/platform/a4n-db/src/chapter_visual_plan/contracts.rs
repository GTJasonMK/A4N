use a4n_db_models::models::chapter::chapter_visual_plan;
use a4n_domain::chapter_visual_plan::{
    ChapterVisualPlanAuthoringMode, ChapterVisualPlanStatus, ChapterVisualPlanV3,
    ExpectedVisualPlanDraft,
};
use a4n_domain::shooting_script::ShootingScriptPlanSyncV1;
use sea_orm::DbErr;

use a4n_domain::chapter::ResolvedChapterSource;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChapterVisualPlan {
    pub record: chapter_visual_plan::Model,
    pub status: ChapterVisualPlanStatus,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub plan: ChapterVisualPlanV3,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChapterVisualPlanSet {
    pub draft: Option<ParsedChapterVisualPlan>,
    pub confirmed: Option<ParsedChapterVisualPlan>,
}

#[derive(Debug, Clone)]
pub struct SaveChapterVisualPlanDraftInput {
    pub source: ResolvedChapterSource,
    pub author_user_id: i64,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub expected: Option<ExpectedVisualPlanDraft>,
    pub expected_source_hash: Option<String>,
    pub plan: ChapterVisualPlanV3,
}

#[derive(Debug, Clone)]
pub struct ConfirmChapterVisualPlanOutcome {
    pub visual_plan: ParsedChapterVisualPlan,
    pub invalidated_project_ids: Vec<i64>,
    /// 同一正文版本的拍摄剧本变基到新方案后的逐镜头同步状态；没有拍摄剧本时为空。
    pub shooting_script_sync: Option<ShootingScriptPlanSyncV1>,
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
