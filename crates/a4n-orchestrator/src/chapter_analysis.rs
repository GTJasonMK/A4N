use a4n_core::{
    chapter_analysis::ensure_chapter_has_analysis_content, error::AppError,
    models::chapter::chapter,
};
use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;
use owner::{build_chapter_analysis_preview_requests, chapter_foreshadowing_resolution_context};

mod application;
mod checkpoint;
mod execution;
mod outputs;
#[path = "chapter_analysis/owner/mod.rs"]
mod owner;
mod persistence;
mod stages;
mod workflow;

pub use outputs::{
    ChapterAnalysisOutputError, chapter_analysis_foreshadowing_drafts,
    retrospective_chapter_outline_input,
};
pub use workflow::{
    CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
    ChapterAnalysisCheckpointIncomplete, ChapterAnalysisResumeBlockReason,
    chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
    chapter_analysis_run_input_matches, chapter_analysis_run_uses_replacement_commit,
    chapter_analysis_workflow_business_key, chapter_analysis_workflow_fingerprint,
    chapter_analysis_workflow_run_input, require_completed_chapter_analysis_phase,
};

pub type ChapterAnalysisProgressReporter<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

pub struct ChapterAnalysisInput<'a> {
    pub chapter: chapter::Model,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub progress: Option<ChapterAnalysisProgressReporter<'a>>,
    /// 活体信号出口：与字段级里程碑取自同一个 token 流。
    pub stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub workflow_run_id: Option<i64>,
}

pub(super) struct ChapterAnalysisProgress<'a> {
    reporter: Option<ChapterAnalysisProgressReporter<'a>>,
    stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ChapterAnalysisProgress<'a> {
    fn new(
        reporter: Option<ChapterAnalysisProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            stream_reporter: None,
            cancel_token,
        }
    }

    fn with_stream_reporter(
        mut self,
        stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    ) -> Self {
        self.stream_reporter = stream_reporter;
        self
    }

    /// 取出活体信号出口，供 LLM 调用处独立持有——避免与 `&mut self` 的进度回调争用借用。
    pub(super) fn stream_reporter(&self) -> Option<&'a (dyn Fn(&str) + Send + Sync)> {
        self.stream_reporter
    }

    pub(super) fn report(
        &mut self,
        current: u32,
        total: u32,
        message: &str,
    ) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter.as_deref_mut() {
            reporter(current, total.max(1), message)?;
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

impl OrchestratorContext {
    pub(crate) async fn build_chapter_analysis_requests(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        let chapter = a4n_db::chapter_content::chapter_by_id_for_analysis(
            self.database.connection(),
            chapter_id,
        )
        .await
        .map_err(chapter_content_edit_error)?;
        ensure_chapter_has_analysis_content(&chapter)?;
        let foreshadowing = self
            .foreshadowing_items_for_rag_ingest(chapter.project_id)
            .await?;
        let resolution_context =
            chapter_foreshadowing_resolution_context(chapter.chapter_number, &foreshadowing);
        build_chapter_analysis_preview_requests(&chapter, prompt_renderer, &resolution_context)
            .await
    }
}

fn chapter_content_edit_error(error: a4n_db::chapter_content::ChapterContentEditError) -> AppError {
    match error {
        a4n_db::chapter_content::ChapterContentEditError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::NotFound(message) => {
            AppError::NotFound(message)
        }
        a4n_db::chapter_content::ChapterContentEditError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        a4n_db::chapter_content::ChapterContentEditError::Database(error) => {
            AppError::Database(error)
        }
    }
}
pub use application::{
    CHAPTER_ANALYSIS_APPLICATION_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
    CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
    CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC, CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
    CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS, ChapterAnalysisApplicationProgress,
    ChapterAnalysisApplicationProgressReporter, ChapterAnalysisHostPort,
    ChapterAnalysisPreviewStep,
};
