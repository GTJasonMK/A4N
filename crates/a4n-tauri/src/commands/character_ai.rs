//! 角色 AI 分析命令。

mod conflict_check;

use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_core::event::channel::TaskType;
use a4n_runtime::AppState;
use a4n_runtime::actions::character_ai as character_ai_actions;
use serde_json::Value;
use tauri::{State, Window};

macro_rules! character_ai_task_command {
    (
        $(#[$meta:meta])*
        $name:ident,
        $scope:path,
        $start_message:expr,
        $error_code:expr,
        ($($arg:ident : $arg_ty:ty),* $(,)?),
        |$value:ident| $success_message:expr,
        |$app_state:ident, $user_id:ident, $project_id:ident, $progress:ident| $run:expr $(,)?
    ) => {
        $(#[$meta])*
        #[tauri::command]
        pub async fn $name(
            window: Window,
            state: State<'_, AppState>,
            user_id: i64,
            project_id: i64,
            $($arg: $arg_ty,)*
            task_id: Option<String>,
        ) -> Result<Value, String> {
            let scope = $scope(state.inner(), user_id, project_id).await?;
            let progress_task = WorkflowProgressTask::new(
                window,
                state.inner(),
                task_id,
                TaskType::LlmCall,
                Some(scope.project_id),
                scope.total,
                $start_message,
            );
            let cancel_token = progress_task.cancel_token();
            let reporter =
                |current, total, message: &str| progress_task.progress(current, total, message);
            let runtime_progress = character_ai_actions::CharacterAiProgress::new(
                Some(&reporter),
                Some(&cancel_token),
            );
            let $app_state = state.inner();
            let $user_id = user_id;
            let $project_id = project_id;
            let $progress = &runtime_progress;
            let result = $run.await;
            progress_task.finish_result(state.inner(), &result, $error_code, |$value| {
                $success_message
            });
            result
        }
    };
}

/// 检测档案与章节冲突：查询角色属性与章节内容，通过 LLM 检查一致性。
#[tauri::command]
pub async fn check_character_conflict(
    window: Window,
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    name: String,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    task_id: Option<String>,
) -> Result<Vec<Value>, String> {
    conflict_check::check_character_conflict_impl(
        window,
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
