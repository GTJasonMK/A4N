use a4n_db_models::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::ChapterShootingScriptRecordSummary;
use a4n_domain::{chapter::ResolvedChapterSource, shooting_script::ChapterShootingScriptV2};
use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedChapterShootingScript {
    pub record: chapter_shooting_script::Model,
    pub script: ChapterShootingScriptV2,
}

pub fn chapter_shooting_script_record_summary(
    record: &ParsedChapterShootingScript,
) -> ChapterShootingScriptRecordSummary {
    ChapterShootingScriptRecordSummary {
        id: record.record.id,
        revision: record.record.revision,
        visual_plan_id: record.record.visual_plan_id,
        visual_plan_version: record.record.visual_plan_version,
        visual_plan_sha256: record.record.visual_plan_sha256.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterVisualPlanIdentity {
    pub id: i64,
    pub version: i32,
    pub sha256: String,
}

#[derive(Debug, Clone)]
pub struct InsertChapterShootingScriptInput {
    pub source: ResolvedChapterSource,
    pub input_fingerprint_sha256: String,
    pub visual_plan: ChapterVisualPlanIdentity,
    pub script: ChapterShootingScriptV2,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct UpdateChapterShootingScriptInput {
    pub record_id: i64,
    pub expected_revision: i32,
    pub input_fingerprint_sha256: String,
    pub visual_plan: ChapterVisualPlanIdentity,
    pub script: ChapterShootingScriptV2,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct DeleteStoredChapterShootingScriptInput {
    pub record_id: i64,
    pub expected_revision: i32,
}

#[derive(Debug, Clone)]
pub struct EditChapterShootingScriptOutcome {
    pub shooting_script: ParsedChapterShootingScript,
    pub invalidated_project_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ShootingScriptDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("{0}")]
    NotFound(String),
    #[error("拍摄剧本 revision 冲突: id={record_id}, expected_revision={expected_revision}")]
    RevisionConflict {
        record_id: i64,
        expected_revision: i32,
    },
    #[error(transparent)]
    Database(#[from] DbErr),
}
