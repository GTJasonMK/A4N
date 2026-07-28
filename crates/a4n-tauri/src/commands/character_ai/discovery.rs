use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::character_ai as character_ai_actions;
use serde_json::Value;
use tauri::State;

/// 从章节内容中发现项目角色，并为缺失角色创建初始档案。
#[tauri::command]
pub async fn discover_character_profiles(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    task_id: Option<String>,
) -> Result<Value, String> {
    let scope = character_ai_actions::character_discovery_task_scope(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
    )
    .await?;
    let progress_task = crate::progress_task::WorkflowProgressTask::new(
        state.inner(),
        task_id,
        a4n_core::event::channel::TaskKind::LlmCall,
        Some(scope.project_id),
        scope.total,
        "正在准备角色发现",
    );
    let cancel_token = progress_task.cancel_token();
    let reporter = |current, total, message: &str| progress_task.progress(current, total, message);
    let runtime_progress =
        character_ai_actions::CharacterAiProgress::new(Some(&reporter), Some(&cancel_token));
    let result = character_ai_actions::discover_character_profiles_for_chapters(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        Some(&runtime_progress),
    )
    .await;
    progress_task.remove(state.inner());
    result
}

#[tauri::command]
pub async fn preview_discover_character_profiles_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<ModelRequestPreview, String> {
    character_ai_actions::preview_discover_character_profiles_request(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
    )
    .await
}
