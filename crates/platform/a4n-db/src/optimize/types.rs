use super::*;
use sea_orm::FromQueryResult;

use crate::rag::RagForeshadowingRow;
use a4n_domain::chapter::{ChapterRecord, PartOutlineRecord};
use a4n_domain::project::{
    OptimizationCharacterProfileRecord, OptimizationSessionRecord, ProjectOptimizationContextRecord,
};
use a4n_domain::project::{OptimizationPatchSetRecord, OptimizationTurnRecord};

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationSessionCreateInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,
    pub status: String,
    pub issues: Option<Value>,
    pub agent_permissions: Option<Value>,
    pub agent_instruction: Option<String>,
    pub fixes_applied: i32,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationSessionUpdateInput {
    pub session: optimization_session::Model,
    pub status: Option<String>,
    pub issues: Option<Option<Value>>,
    pub agent_permissions: Option<Option<Value>>,
    pub agent_instruction: Option<Option<String>>,
    pub fixes_applied: Option<i32>,
    pub summary: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationSessionRecordUpdateInput {
    pub session: OptimizationSessionRecord,
    pub status: Option<String>,
    pub issues: Option<Option<Value>>,
    pub agent_permissions: Option<Option<Value>>,
    pub agent_instruction: Option<Option<String>>,
    pub fixes_applied: Option<i32>,
    pub summary: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationSessionMessageInsertInput {
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationTurnCreateInput {
    pub session_id: i64,
    pub agent_turn_id: Option<i64>,
    pub user_text: String,
    pub status: String,
    pub permissions_snapshot: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationTurnUpdateInput {
    pub turn: OptimizationTurnRecord,
    pub status: Option<String>,
    pub final_reply: Option<Option<String>>,
    pub error: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationTurnEventAppendInput {
    pub turn_id: i64,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationPatchCreateInput {
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: String,
    pub old_text: String,
    pub new_text: String,
    pub reason: String,
    pub risk: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationPatchSetCreateInput {
    pub turn_id: i64,
    pub session_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub status: String,
    pub summary: String,
    pub patch_decision: String,
    pub patch_apply_status: String,
    pub patches: Vec<OptimizationPatchCreateInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompleteOptimizationPatchTurnInput {
    pub session_id: i64,
    pub turn_id: i64,
    pub assistant_content: String,
    pub tool_content: String,
    pub tool_calls: Option<Value>,
    pub patch_set: OptimizationPatchSetCreateInput,
    pub final_reply: String,
    pub session_summary: String,
    pub tool_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompleteOptimizationPatchTurnResult {
    pub turn: OptimizationTurnRecord,
    pub patch_set: Option<OptimizationPatchSetRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptimizationContextReadInput {
    pub project_id: i64,
    pub chapter_id: i64,
    pub read_project_detail: bool,
    pub read_blueprint: bool,
    pub read_outlines: bool,
    pub read_character_profiles: bool,
    pub read_foreshadowing: bool,
}

pub type OptimizationBlueprintRows = Value;

#[derive(Debug, Clone, FromQueryResult)]
pub struct OptimizationChapterSummaryRecord {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub word_count: i32,
    pub analysis_data: Option<Value>,
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct OptimizationChapterOutlineRecord {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct OptimizationContextRows {
    pub current_chapter: ChapterRecord,
    pub current_chapter_outline: Option<OptimizationChapterOutlineRecord>,
    pub project: Option<ProjectOptimizationContextRecord>,
    pub blueprint: Option<OptimizationBlueprintRows>,
    pub part_outlines: Vec<PartOutlineRecord>,
    pub chapter_outlines: Vec<OptimizationChapterOutlineRecord>,
    pub character_profiles: Vec<OptimizationCharacterProfileRecord>,
    pub foreshadowing_index: Vec<RagForeshadowingRow>,
    pub previous_chapters: Vec<OptimizationChapterSummaryRecord>,
    pub next_chapters: Vec<OptimizationChapterSummaryRecord>,
}

#[derive(Debug, Clone)]
pub struct OptimizationChapterPatchWriteInput {
    pub chapter: ChapterRecord,
    pub content: String,
    pub word_count: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizeDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl From<crate::generation::GenerationLifecycleGateError> for OptimizeDbError {
    fn from(error: crate::generation::GenerationLifecycleGateError) -> Self {
        use crate::generation::GenerationLifecycleGateError as Error;
        match error {
            Error::InvalidArgument(message) => Self::InvalidArgument(message),
            Error::NotFound(message) => Self::NotFound(message),
            Error::Blocked(message) => Self::InvalidState {
                current: "project_lifecycle_gate".to_string(),
                operation: message,
            },
            Error::Corrupt(message) => Self::Database(DbErr::Custom(message)),
            Error::Database(error) => Self::Database(error),
        }
    }
}
