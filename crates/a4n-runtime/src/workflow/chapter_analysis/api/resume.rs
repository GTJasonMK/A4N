use a4n_api_contract::chapter::ChapterSummary;
use a4n_error::AppError;
use a4n_orchestrator::chapter_analysis::ChapterAnalysisApplicationProgress;

use super::super::{ChapterAnalysisProgress, host_port::RuntimeChapterAnalysisHostPort};
use crate::{actions::character_ai::RuntimeCharacterProfileSyncStageTracePort, state::AppState};

pub async fn preflight_resume_chapter_analysis_sync(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ChapterSummary, String> {
    state
        .chapter_analysis_resume_application()
        .preflight_resume(user_id, chapter_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn resume_chapter_analysis_sync_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<ChapterSummary, String> {
    let prompt_renderer = state;
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
    let reporter = |current: u32, total: u32, message: &str| {
        if let Some(progress) = progress {
            progress
                .report(current, total, message, "章节分析继续同步已停止")
                .map_err(|error| {
                    if progress
                        .cancel_token
                        .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
                    {
                        AppError::Cancelled
                    } else {
                        AppError::InvalidArgument(error)
                    }
                })?;
        }
        Ok(())
    };
    state
        .chapter_analysis_resume_application()
        .resume_sync(
            user_id,
            chapter_id,
            prompt_renderer,
            &host_port,
            ChapterAnalysisApplicationProgress {
                reporter: Some(&reporter),
                stream_reporter: progress.and_then(|progress| progress.stream_reporter),
                cancel_token: progress.and_then(|progress| progress.cancel_token),
                active_task_id: progress.and_then(|progress| progress.task_id),
            },
            Some(&trace_port),
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn resume_chapter_analysis_workflow_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<(), String> {
    let prompt_renderer = state;
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
    state
        .chapter_analysis_resume_application()
        .resume_workflow_run(
            user_id,
            workflow_run_id,
            prompt_renderer,
            &host_port,
            Some(&trace_port),
        )
        .await
        .map_err(|error| error.to_string())
}
