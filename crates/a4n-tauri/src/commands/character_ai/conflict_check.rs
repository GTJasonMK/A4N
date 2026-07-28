use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::AppState;
use a4n_runtime::host_api::character_conflict as character_conflict_actions;
use serde_json::Value;
use tauri::State;

pub(super) async fn check_character_conflict_impl(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    task_id: Option<String>,
) -> Result<Vec<Value>, String> {
    let scope = character_conflict_actions::character_conflict_task_scope(
        state.inner(),
        user_id,
        project_id,
        &name,
        start_chapter,
        end_chapter,
    )
    .await
    .map_err(|err| err.to_string())?;
    let progress = WorkflowProgressTask::new(
        state.inner(),
        task_id,
        TaskKind::LlmCall,
        Some(scope.project_id),
        scope.total,
        "正在准备角色一致性检查",
    );
    let reporter = |current, total, message: &str| {
        progress.progress(current, total, message);
        progress
            .check_cancelled("角色一致性检查已停止")
            .map_err(a4n_core::error::AppError::InvalidArgument)
    };
    let result = character_conflict_actions::check_character_conflict(
        &state,
        user_id,
        project_id,
        &name,
        start_chapter,
        end_chapter,
        &reporter,
    )
    .await
    .map_err(|err| err.to_string());
    progress.remove(state.inner());
    result
}

pub(super) async fn preview_character_conflict_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<ModelRequestPreview, String> {
    character_conflict_actions::preview_character_conflict_request(
        &state,
        user_id,
        project_id,
        &name,
        start_chapter,
        end_chapter,
    )
    .await
    .map_err(|err| err.to_string())
}
