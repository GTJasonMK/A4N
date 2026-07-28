use super::conflict_check;
use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use serde_json::Value;
use tauri::State;

/// 检测档案与章节冲突：查询角色属性与章节内容，通过 LLM 检查一致性。
#[tauri::command]
pub async fn check_character_conflict(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    task_id: Option<String>,
) -> Result<Vec<Value>, String> {
    conflict_check::check_character_conflict_impl(
        state,
        user_id,
        project_id,
        name,
        start_chapter,
        end_chapter,
        task_id,
    )
    .await
}

#[tauri::command]
pub async fn preview_check_character_conflict_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<ModelRequestPreview, String> {
    conflict_check::preview_character_conflict_request(
        state,
        user_id,
        project_id,
        name,
        start_chapter,
        end_chapter,
    )
    .await
}
