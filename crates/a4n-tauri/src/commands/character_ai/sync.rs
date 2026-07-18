use super::task::character_ai_task_command;
use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::character_ai as character_ai_actions;
use serde_json::Value;
use tauri::State;

character_ai_task_command!(
    /// 从章节同步更新角色：按档案断点分析章节，将角色变化落为事件、行为、删除候选、状态索引与快照。
    sync_character_from_chapters,
    character_ai_actions::character_profile_sync_task_scope,
    "正在准备角色同步",
    "character_sync_failed",
    (name: String, chapter_number: i32),
    |value| {
        format!(
            "角色同步完成：{}，同步至第 {} 章",
            value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(name.as_str()),
            value
                .get("synced_chapter")
                .and_then(Value::as_i64)
                .unwrap_or(i64::from(chapter_number))
        )
    },
    |state, user_id, project_id, progress| {
        character_ai_actions::sync_character_profile_from_chapters(
            state,
            user_id,
            project_id,
            &name,
            chapter_number,
            Some(progress),
        )
    },
);

#[tauri::command]
pub async fn preview_sync_character_from_chapters_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
    chapter_number: i32,
) -> Result<ModelRequestPreview, String> {
    character_ai_actions::preview_sync_character_from_chapters_request(
        state.inner(),
        user_id,
        project_id,
        &name,
        chapter_number,
    )
    .await
}

character_ai_task_command!(
    /// 同步项目下所有活跃角色档案。单个角色失败不阻断其他角色，并在结果中显式返回失败项。
    sync_all_character_profiles,
    character_ai_actions::character_profile_sync_all_task_scope,
    "正在准备批量角色同步",
    "character_sync_all_failed",
    (chapter_number: i32),
    |value| {
        format!(
            "批量角色同步完成：成功 {} 个，跳过 {} 个，失败 {} 个",
            value
                .get("synced")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            value
                .get("skipped")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
            value
                .get("failed")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        )
    },
    |state, user_id, project_id, progress| {
        character_ai_actions::sync_all_character_profiles_to_chapter(
            state,
            user_id,
            project_id,
            chapter_number,
            Some(progress),
        )
    },
);

#[tauri::command]
pub async fn preview_sync_all_character_profiles_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
) -> Result<ModelRequestPreview, String> {
    character_ai_actions::preview_sync_all_character_profiles_request(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
    )
    .await
}
