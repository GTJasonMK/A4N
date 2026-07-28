use a4n_core::event::channel::TaskKind;
use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, finish_web_task, i64_arg, optional_i32_field, optional_string_field,
    register_web_task, sanitize_optional_task_id, scene_profile_sync_actions, string_action_result,
    web_fire_and_forget_progress_reporter,
};

pub(in crate::routes::commands) async fn sync_scene_profiles(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = optional_i32_field(args, "chapterNumber")?;
    let scope = scene_profile_sync_actions::scene_profile_sync_task_scope(
        state.runtime(),
        user_id,
        project_id,
    )
    .await
    .map_err(ApiError::bad_request)?;

    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskKind::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备场景档案同步",
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let reporter = web_fire_and_forget_progress_reporter(task.clone());
    let progress = scene_profile_sync_actions::SceneProfileSyncProgress::new(
        Some(&reporter),
        cancel_token.as_ref(),
    );
    let result = scene_profile_sync_actions::sync_scene_profiles_for_chapters(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
        Some(&progress),
    )
    .await;
    let value = finish_web_task(
        state,
        task_id,
        string_action_result(
            result,
            task.as_ref().is_some_and(|handle| handle.is_cancelled()),
        ),
    )
    .await?;
    Ok(value)
}
