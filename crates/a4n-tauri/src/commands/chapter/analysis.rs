use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_runtime::AppState;
use a4n_runtime::host_api::chapter_content::{ChapterSummary, chapter_summary};
use a4n_runtime::workflow::{
    self as chapter_workflow, CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisProgress,
    GenerateChapterAnalysisInput,
};
use tauri::{Emitter, State, Window};

tauri_chapter_progress_command!(
    generate_chapter_analysis(input: GenerateChapterAnalysisInput) -> ChapterSummary,
    {
        let ch = chapter_workflow::preflight_generate_chapter_analysis(
            state.inner(),
            user_id,
            input.chapter_id,
        )
        .await?;
    },
    project_id: ch.project_id,
    total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
    "正在准备章节结构化分析",
    |window, state, user_id, progress| generate_chapter_analysis_inner(
        state.inner(),
        user_id,
        progress,
        input.chapter_id,
    ),
    |chapter| {
        emit_chapter_analysis_updated(&window, chapter, "章节分析完成事件发送失败");
    },
);

tauri_chapter_progress_command!(
    resume_chapter_analysis_sync(input: GenerateChapterAnalysisInput) -> ChapterSummary,
    {
        let ch = chapter_workflow::preflight_resume_chapter_analysis_sync(
            state.inner(),
            user_id,
            input.chapter_id,
        )
        .await?;
    },
    project_id: ch.project_id,
    total: CHAPTER_ANALYSIS_PROGRESS_TOTAL,
    "正在准备继续同步章节分析",
    |window, state, user_id, progress| resume_chapter_analysis_sync_inner(
        state.inner(),
        user_id,
        progress,
        input.chapter_id,
    ),
    |chapter| {
        emit_chapter_analysis_updated(&window, chapter, "章节分析继续同步事件发送失败");
    },
);

#[tauri::command]
pub async fn preview_chapter_analysis_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterAnalysisInput,
) -> Result<ModelRequestPreview, String> {
    chapter_workflow::preview_chapter_analysis_request(state.inner(), user_id, input.chapter_id)
        .await
}

#[tauri::command]
pub async fn preview_resume_chapter_analysis_sync_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterAnalysisInput,
) -> Result<ModelRequestPreview, String> {
    chapter_workflow::preview_resume_chapter_analysis_sync_request(
        state.inner(),
        user_id,
        input.chapter_id,
    )
    .await
}

fn emit_chapter_analysis_updated(window: &Window, chapter: &ChapterSummary, warn_message: &str) {
    if let Err(err) = window.emit(
        "chapter-analysis-updated",
        serde_json::json!({
            "project_id": chapter.project_id,
            "chapter_id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "updated_at": chapter.updated_at,
        }),
    ) {
        tracing::warn!(
            project_id = chapter.project_id,
            chapter_id = chapter.id,
            error = %err,
            "{warn_message}"
        );
    }
}

async fn generate_chapter_analysis_inner(
    state: &AppState,
    user_id: i64,
    progress: &WorkflowProgressTask,
    chapter_id: i64,
) -> Result<ChapterSummary, String> {
    let cancel_token = progress.cancel_token();
    let reporter = |current, total, message: &str| progress.progress(current, total, message);
    let stream_reporter = |text: &str| progress.stream_text(text);
    let runtime_progress =
        ChapterAnalysisProgress::new(Some(&reporter), Some(&cancel_token), progress.task_id())
            .with_stream_reporter(Some(&stream_reporter));
    let updated = chapter_workflow::generate_chapter_analysis_with_progress(
        state,
        user_id,
        chapter_id,
        Some(&runtime_progress),
    )
    .await?;
    Ok(chapter_summary(&updated))
}

async fn resume_chapter_analysis_sync_inner(
    state: &AppState,
    user_id: i64,
    progress: &WorkflowProgressTask,
    chapter_id: i64,
) -> Result<ChapterSummary, String> {
    let cancel_token = progress.cancel_token();
    let reporter = |current, total, message: &str| progress.progress(current, total, message);
    let stream_reporter = |text: &str| progress.stream_text(text);
    let runtime_progress =
        ChapterAnalysisProgress::new(Some(&reporter), Some(&cancel_token), progress.task_id())
            .with_stream_reporter(Some(&stream_reporter));
    let updated = chapter_workflow::resume_chapter_analysis_sync_with_progress(
        state,
        user_id,
        chapter_id,
        Some(&runtime_progress),
    )
    .await?;
    Ok(chapter_summary(&updated))
}
