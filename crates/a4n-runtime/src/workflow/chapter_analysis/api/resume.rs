use a4n_core::{
    error::AppError,
    models::{chapter::chapter, generation::workflow_run},
};
use a4n_orchestrator::chapter_analysis::ChapterAnalysisApplicationProgress;

use super::super::{ChapterAnalysisProgress, host_port::RuntimeChapterAnalysisHostPort};
use crate::{
    actions::{access, app_error_message, character_ai::RuntimeCharacterProfileSyncStageTracePort},
    prompt_renderer::RuntimePromptRenderer,
    state::AppState,
};

pub async fn preflight_resume_chapter_analysis_sync(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<chapter::Model, String> {
    let chapter = state
        .chapter_analysis()
        .preflight_chapter_analysis_validated(chapter_id, true)
        .await
        .map_err(|error| error.to_string())?;
    access::verify_project_editor(state, user_id, chapter.project_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(chapter)
}

pub async fn resume_chapter_analysis_sync_with_progress(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    progress: Option<&ChapterAnalysisProgress<'_>>,
) -> Result<chapter::Model, String> {
    preflight_resume_chapter_analysis_sync(state, user_id, chapter_id).await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
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
        .chapter_analysis()
        .resume_chapter_analysis_sync_validated(
            chapter_id,
            &prompt_renderer,
            &host_port,
            ChapterAnalysisApplicationProgress {
                reporter: Some(&reporter),
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
    run: &workflow_run::Model,
) -> Result<(), String> {
    let chapter_id = run
        .chapter_id
        .ok_or_else(|| "章节结构化分析工作流缺少 chapter_id".to_string())?;
    let chapter = state
        .project_chapter_records()
        .chapter_by_id(chapter_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("章节不存在: chapter_id={chapter_id}"))?;
    access::verify_project_editor(state, user_id, chapter.project_id)
        .await
        .map_err(|error| error.to_string())?;

    let prompt_renderer = RuntimePromptRenderer::new(state);
    let host_port = RuntimeChapterAnalysisHostPort::new(state);
    let trace_port = RuntimeCharacterProfileSyncStageTracePort::new(state.log_dir());
    state
        .chapter_analysis()
        .resume_chapter_analysis_workflow_run_validated(
            run,
            &prompt_renderer,
            &host_port,
            Some(&trace_port),
        )
        .await
        .map_err(|error| error.to_string())
}
