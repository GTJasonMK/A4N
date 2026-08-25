use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::chapter_visual_plan::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanSourceInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SaveChapterVisualPlanDraftInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub record_id: Option<i64>,
    pub expected_revision: Option<i32>,
    pub plan: ChapterVisualPlanV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanRecordInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterVisualPlanDraftInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub user_direction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProposeChapterVisualPlanRevisionInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub base_revision: i32,
    pub instruction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplyChapterVisualPlanProposalInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub base_revision: i32,
    pub proposal: ChapterVisualPlanV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChapterVisualPlanRecordSummary {
    pub id: i64,
    pub plan_version: i32,
    pub revision: i32,
    pub status: ChapterVisualPlanStatus,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub source_content_sha256: String,
    pub plan_sha256: String,
    pub confirmed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanView {
    pub record: ChapterVisualPlanRecordSummary,
    pub plan: ChapterVisualPlanV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanSetView {
    pub source: ChapterVisualPlanSourceView,
    pub starter_plan: ChapterVisualPlanV1,
    pub draft: Option<ChapterVisualPlanView>,
    pub confirmed: Option<ChapterVisualPlanView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChapterVisualPlanSourceSpanView {
    pub id: String,
    pub text: String,
    pub paragraph_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChapterVisualPlanSourceView {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub content_sha256: String,
    pub source_spans: Vec<ChapterVisualPlanSourceSpanView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ConfirmedChapterVisualPlanView {
    pub visual_plan: ChapterVisualPlanView,
    pub invalidated_project_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanProposalView {
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub base_revision: i32,
    pub proposal: ChapterVisualPlanV1,
}
