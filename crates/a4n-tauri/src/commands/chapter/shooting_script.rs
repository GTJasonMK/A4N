use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::host_api::shooting_script_records as shooting_script_records_actions;
use shooting_script_records_actions::{
    CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptView, EditChapterShootingScriptRecordInput,
    EditedChapterShootingScriptView, GenerateChapterShootingScriptInput,
    GetChapterShootingScriptInput,
};
use tauri::State;

#[tauri::command]
pub async fn get_chapter_shooting_script(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetChapterShootingScriptInput,
) -> Result<Option<ChapterShootingScriptView>, String> {
    shooting_script_records_actions::get_chapter_shooting_script(state.inner(), user_id, input)
        .await
}

#[tauri::command]
pub async fn get_chapter_shooting_script_unit_progress(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetChapterShootingScriptInput,
) -> Result<Option<ChapterShootingScriptUnitProgressView>, String> {
    shooting_script_records_actions::get_chapter_shooting_script_unit_progress(
        state.inner(),
        user_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn generate_next_chapter_shooting_script_unit(
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    task_id: Option<String>,
) -> Result<ChapterShootingScriptUnitProgressView, String> {
    let progress_task = WorkflowProgressTask::new(
        state.inner(),
        Some(user_id),
        task_id,
        TaskKind::LlmCall,
        Some(input.project_id),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        "正在准备下一个场景改编",
    );
    let cancel_token = progress_task.cancel_token();
    let reporter = |current, total, message: &str| {
        progress_task.progress(current, total, message);
        if progress_task.check_cancelled("场景改编已停止").is_err() {
            return Err(a4n_core::error::AppError::Cancelled);
        }
        Ok(())
    };
    let stream_reporter = |text: &str| progress_task.stream_text(text);
    let result = shooting_script_records_actions::generate_next_chapter_shooting_script_unit(
        state.inner(),
        user_id,
        input,
        &reporter,
        Some(&stream_reporter),
        Some(&cancel_token),
        progress_task.task_id(),
    )
    .await;
    progress_task.remove(state.inner());
    result
}

#[tauri::command]
pub async fn generate_chapter_shooting_script(
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    task_id: Option<String>,
) -> Result<ChapterShootingScriptView, String> {
    let progress_task = WorkflowProgressTask::new(
        state.inner(),
        Some(user_id),
        task_id,
        TaskKind::LlmCall,
        Some(input.project_id),
        CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        "正在准备剧本改编",
    );
    let cancel_token = progress_task.cancel_token();
    let reporter = |current, total, message: &str| {
        progress_task.progress(current, total, message);
        if progress_task.check_cancelled("剧本改编已停止").is_err() {
            return Err(a4n_core::error::AppError::Cancelled);
        }
        Ok(())
    };
    let stream_reporter = |text: &str| progress_task.stream_text(text);
    let result = shooting_script_records_actions::generate_chapter_shooting_script(
        state.inner(),
        user_id,
        input,
        &reporter,
        Some(&stream_reporter),
        Some(&cancel_token),
        progress_task.task_id(),
    )
    .await;
    progress_task.remove(state.inner());
    result
}

#[tauri::command]
pub async fn edit_chapter_shooting_script(
    state: State<'_, AppState>,
    user_id: i64,
    input: EditChapterShootingScriptRecordInput,
) -> Result<EditedChapterShootingScriptView, String> {
    shooting_script_records_actions::edit_chapter_shooting_script(state.inner(), user_id, input)
        .await
}
