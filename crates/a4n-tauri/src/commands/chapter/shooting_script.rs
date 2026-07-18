use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::host_api::shooting_script_records as shooting_script_records_actions;
use shooting_script_records_actions::{
    CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL, ChapterShootingScriptView,
    EditChapterShootingScriptRecordInput, EditedChapterShootingScriptView,
    GenerateChapterShootingScriptInput, GetChapterShootingScriptInput,
};
use tauri::{State, Window};

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
pub async fn generate_chapter_shooting_script(
    window: Window,
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    task_id: Option<String>,
) -> Result<ChapterShootingScriptView, String> {
    let progress_task = WorkflowProgressTask::new(
        window,
        state.inner(),
        task_id,
        TaskType::LlmCall,
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
    let result = shooting_script_records_actions::generate_chapter_shooting_script(
        state.inner(),
        user_id,
        input,
        &reporter,
        Some(&cancel_token),
    )
    .await;
    progress_task.finish_result(
        state.inner(),
        &result,
        "chapter_shooting_script_failed",
        |script| format!("第 {} 章拍摄剧本已生成", script.record.source_chapter_id),
    );
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
