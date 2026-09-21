//! Runtime-facing contracts for project import analysis.

use a4n_api_contract::import::ImportProjectAnalysisSummary;
use a4n_chapter_analysis_host_contract::{ChapterAnalysisHostPort, ChapterAnalysisRequestStep};
use a4n_character_host_contract::CharacterProfileSyncStageTracePort;
use a4n_error::AppError;
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;

#[async_trait]
pub trait ImportAnalysisPreviewApplicationPort: Send + Sync {
    async fn preview(
        &self,
        user_id: i64,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError>;
}

#[async_trait]
pub trait ImportAnalysisGenerationApplicationPort: Send + Sync {
    async fn run(
        &self,
        user_id: i64,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        chapter_analysis_host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ImportProjectAnalysisSummary, AppError>;
}
