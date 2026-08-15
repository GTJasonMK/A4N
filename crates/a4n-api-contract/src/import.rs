use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_PROJECT_TEXT_UPLOAD_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectTextUploadInput {
    pub project_id: i64,
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportAnalysisStatus {
    Analyzing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportAnalysisStageKey {
    GeneratingAnalysisData,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportAnalysisStageStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportAnalysisStageProgress {
    pub name: String,
    pub completed: usize,
    pub total: usize,
    pub status: ImportAnalysisStageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportAnalysisStages {
    pub generating_analysis_data: ImportAnalysisStageProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectImportAnalysisProgress {
    pub status: ImportAnalysisStatus,
    pub current_stage: ImportAnalysisStageKey,
    pub stages: ImportAnalysisStages,
    pub message: String,
    pub error: Option<String>,
    pub overall_progress: u8,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub failed_at: Option<String>,
    pub cancelled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectAnalysisSummary {
    pub progress: ProjectImportAnalysisProgress,
    pub analyzed_chapters: usize,
}
