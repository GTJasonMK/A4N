use a4n_api_contract::profile::SyncSceneProfilesInput;
use a4n_core::event::channel::TaskKind;
use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, finish_web_task, optional_string_field, register_web_task,
    sanitize_optional_task_id, scene_profile_sync_actions,
    strict_optional_task_runtime_command_input, string_action_result, to_value,
    web_fire_and_forget_progress_reporter, web_stream_reporter,
};

pub(in crate::routes::commands) async fn sync_scene_profiles(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: SyncSceneProfilesInput =
        strict_optional_task_runtime_command_input(args, "sync_scene_profiles")?;

    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        input.project_id,
        scene_profile_sync_actions::SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
        "正在准备场景档案同步",
    );
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let reporter = web_fire_and_forget_progress_reporter(task.clone());
    // 与桌面端对称：活体信号必须双端一致，否则浏览器端会缺失流式尾巴。
    let stream_reporter = web_stream_reporter(task.clone());
    let progress = scene_profile_sync_actions::SceneProfileSyncProgress::new(
        Some(&reporter),
        cancel_token.as_ref(),
        task_id.as_deref(),
    )
    .with_stream_reporter(Some(&stream_reporter));
    let result = scene_profile_sync_actions::sync_scene_profiles(
        state.runtime(),
        user_id,
        input,
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
    )?;
    to_value(value)
}
