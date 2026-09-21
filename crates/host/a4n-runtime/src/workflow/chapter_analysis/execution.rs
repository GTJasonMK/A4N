use a4n_api_contract::chapter::ChapterAnalysisMutationResult;
use a4n_chapter_analysis_host_contract::ChapterAnalysisApplicationProgress;
use a4n_error::AppError;

use super::{ChapterAnalysisProgress, host_port::RuntimeChapterAnalysisHostPort};
use crate::state::AppState;

pub(super) async fn run_chapter_analysis_application(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<ChapterAnalysisMutationResult, AppError> {
    let prompt_renderer = state;
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = crate::actions::character_ai::RuntimeCharacterProfileSyncStageTracePort::new(
        state.log_dir(),
    );
    let reporter = |current: u32, total: u32, message: &str| {
        if let Some(progress) = progress {
            progress.report(current, total, message)?;
        }
        Ok(())
    };
    let application_progress = ChapterAnalysisApplicationProgress {
        reporter: Some(&reporter),
        stream_reporter: progress.and_then(|progress| progress.stream_reporter),
        cancel_token: progress.and_then(|progress| progress.cancel_token),
        active_task_id: progress.and_then(|progress| progress.task_id),
    };
    state
        .chapter_analysis_generation_application()
        .generate(
            user_id,
            chapter_id,
            prompt_renderer,
            &host_port,
            application_progress,
            Some(&trace_port),
        )
        .await
}
