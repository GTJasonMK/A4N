use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

mod application;
mod checkpoint;
mod execution;
mod outputs;
#[path = "chapter_analysis/owner/mod.rs"]
mod owner;
mod persistence {
    mod checkpoints;
    mod trace;
}
mod stages {
    mod contracts;
    mod entrypoints;
    mod runner;
}
mod workflow;

pub use outputs::{ChapterAnalysisOutputError, chapter_analysis_foreshadowing_drafts};
pub(crate) use workflow::{
    CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
};
pub use workflow::{
    ChapterAnalysisResumeBlockReason, chapter_analysis_phase_fingerprint,
    chapter_analysis_resume_block_reason, chapter_analysis_run_input_matches,
    chapter_analysis_run_uses_replacement_commit, chapter_analysis_workflow_business_key,
    chapter_analysis_workflow_fingerprint, chapter_analysis_workflow_run_input,
    require_completed_chapter_analysis_phase,
};

pub(crate) type ChapterAnalysisProgressReporter<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

pub(crate) struct ChapterAnalysisInput<'a> {
    pub chapter: ChapterRecord,
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

pub(crate) use owner::build_chapter_analysis_initial_request;

pub(crate) use a4n_chapter_analysis_host_contract::{
    ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort,
};
