//! Runtime-facing contracts for chapter analysis execution and recovery.

use a4n_api_contract::chapter::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisMutationResult, ChapterDerivedCleanupHostPort,
    ChapterSummary,
};
use a4n_character_host_contract::CharacterProfileSyncStageTracePort;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt_contract::rendering::PromptRenderer;
use a4n_scene_host_contract::SceneProfileSyncHostPort;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

pub type ChapterAnalysisApplicationProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone)]
pub struct ChapterAnalysisRequestStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub request: ChatRequest,
}

#[derive(Clone, Copy, Default)]
pub struct ChapterAnalysisApplicationProgress<'a> {
    pub reporter: Option<ChapterAnalysisApplicationProgressReporter<'a>>,
    pub stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub active_task_id: Option<&'a str>,
}

impl ChapterAnalysisApplicationProgress<'_> {
    pub fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, CHAPTER_ANALYSIS_PROGRESS_TOTAL, message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }

    pub fn subprogress(
        &self,
        current: u32,
        total: u32,
        span_start: u32,
        span_end: u32,
        message: &str,
    ) -> Result<(), AppError> {
        let total = total.max(1);
        let span_width = span_end
            .checked_sub(span_start)
            .ok_or_else(|| AppError::InvalidArgument("章节分析进度区间顺序无效".to_string()))?;
        let offset = u64::from(span_width) * u64::from(current.min(total)) / u64::from(total);
        let mapped = span_start
            .checked_add(u32::try_from(offset).map_err(|_| {
                AppError::InvalidArgument("章节分析子进度超出 u32 范围".to_string())
            })?)
            .ok_or_else(|| AppError::InvalidArgument("章节分析进度溢出".to_string()))?;
        if let Some(reporter) = self.reporter {
            reporter(mapped, CHAPTER_ANALYSIS_PROGRESS_TOTAL, message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}

#[async_trait]
pub trait ChapterAnalysisHostPort: ChapterDerivedCleanupHostPort + Send + Sync {
    fn scene_profile_sync_host_port(&self) -> &dyn SceneProfileSyncHostPort;
}

#[derive(Debug)]
pub struct ChapterAnalysisRequestPreview {
    pub title: String,
    pub steps: Vec<ChapterAnalysisRequestStep>,
}

#[async_trait]
pub trait ChapterAnalysisGenerationApplicationPort: Send + Sync {
    async fn preflight_generate(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError>;

    async fn generate(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterAnalysisMutationResult, AppError>;
}

#[async_trait]
pub trait ChapterAnalysisResumeApplicationPort: Send + Sync {
    async fn preflight_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError>;

    async fn resume_sync(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterSummary, AppError>;

    async fn resume_workflow_run(
        &self,
        user_id: i64,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait ChapterAnalysisGenerationPreviewApplicationPort: Send + Sync {
    async fn preview(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError>;
}

#[async_trait]
pub trait ChapterAnalysisResumePreviewApplicationPort: Send + Sync {
    async fn preview_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError>;
}
