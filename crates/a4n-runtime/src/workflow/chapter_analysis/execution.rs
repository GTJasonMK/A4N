use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_orchestrator::chapter_analysis::ChapterAnalysisApplicationProgress;

use super::{ChapterAnalysisProgress, host_port::RuntimeChapterAnalysisHostPort};
use crate::{actions::access, prompt_renderer::RuntimePromptRenderer, state::AppState};

pub async fn generate_and_sync_chapter_analysis_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    let chapter = state
        .chapter_analysis()
        .preflight_chapter_analysis_validated(chapter_id, false)
        .await
        .map_err(|error| error.to_string())?;
    access::verify_project_editor(state, user_id, chapter.project_id)
        .await
        .map_err(|error| error.to_string())?;
    run_chapter_analysis_application(state, chapter_id, progress, false).await
}

pub(super) async fn run_chapter_analysis_application(
    state: &AppState,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
    cleanup_before_analysis: bool,
) -> Result<chapter::Model, String> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = crate::actions::character_ai::RuntimeCharacterProfileSyncStageTracePort::new(
        state.log_dir(),
    );
    let reporter = |current: u32, total: u32, message: &str| {
        if let Some(progress) = progress {
            progress
                .report(current, total, message, "章节结构化分析已停止")
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
    let application_progress = ChapterAnalysisApplicationProgress {
        reporter: Some(&reporter),
        stream_reporter: progress.and_then(|progress| progress.stream_reporter),
        cancel_token: progress.and_then(|progress| progress.cancel_token),
        active_task_id: progress.and_then(|progress| progress.task_id),
    };
    let result = if cleanup_before_analysis {
        state
            .chapter_analysis()
            .run_chapter_analysis_validated(
                chapter_id,
                &prompt_renderer,
                &host_port,
                application_progress,
                Some(&trace_port),
            )
            .await
    } else {
        state
            .chapter_analysis()
            .run_chapter_analysis_after_content_postprocess_validated(
                chapter_id,
                &prompt_renderer,
                &host_port,
                application_progress,
                Some(&trace_port),
            )
            .await
    };
    result.map_err(|error| error.to_string())
}
