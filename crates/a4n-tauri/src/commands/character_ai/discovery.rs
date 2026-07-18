use super::task::character_ai_task_command;
use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::character_ai as character_ai_actions;
use serde_json::Value;
use tauri::State;

character_ai_task_command!(
    /// 从章节内容中发现项目角色，并为缺失角色创建初始档案。
    discover_character_profiles,
    character_ai_actions::character_discovery_task_scope,
    "正在准备角色发现",
    "character_discovery_failed",
    (chapter_number: Option<i32>),
    |value| {
        let created_count = value
            .get("created")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let existing_count = value
            .get("existing")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let discovered_count = value
            .get("discovered_count")
            .and_then(Value::as_u64)
            .map(|count| count as usize)
            .unwrap_or(created_count + existing_count);
        format!(
            "角色发现完成：识别 {discovered_count} 个，新建 {created_count} 个，已存在 {existing_count} 个"
        )
    },
    |state, user_id, project_id, progress| {
        character_ai_actions::discover_character_profiles_for_chapters(
            state,
            user_id,
            project_id,
            chapter_number,
            Some(progress),
        )
    },
);

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
