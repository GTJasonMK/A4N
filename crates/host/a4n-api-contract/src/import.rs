use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_import_contract::MAX_PROJECT_TEXT_UPLOAD_BYTES;

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

/// Host-facing read boundary for project import-analysis progress.
///
/// Analysis execution and cancellation remain with the import-analysis owner.
#[async_trait::async_trait]
pub trait ImportAnalysisProgressApplicationPort: Send + Sync {
    async fn progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<ProjectImportAnalysisProgress>, a4n_error::AppError>;
}

/// Host-facing cancellation boundary for project import analysis.
///
/// Analysis execution remains with the import-analysis owner.
#[async_trait::async_trait]
pub trait ImportAnalysisCancellationApplicationPort: Send + Sync {
    async fn cancel(&self, user_id: i64, project_id: i64) -> Result<(), a4n_error::AppError>;
}
