use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, i64_arg, nullable_i64_arg, optimize_actions, string_arg,
};
use super::adapters::{optimize_web_session_command, optimize_web_session_unit_command};

pub(in crate::routes::commands) async fn list_optimize_sessions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = nullable_i64_arg(args, "chapterId")?;
    Ok(json!(
        optimize_actions::list_optimize_sessions(state.runtime(), user_id, project_id, chapter_id,)
            .await
            .map_err(ApiError::bad_request)?
    ))
}

optimize_web_session_command!(get_optimize_session, optimize_actions::get_optimize_session);
optimize_web_session_command!(list_optimize_turns, optimize_actions::list_optimize_turns);
optimize_web_session_command!(
    list_optimize_patch_sets,
    optimize_actions::list_optimize_patch_sets,
);
optimize_web_session_unit_command!(
    reject_pending_optimize_patch_sets,
    optimize_actions::reject_pending_optimize_patch_sets,
);
optimize_web_session_unit_command!(
    delete_optimize_session,
    optimize_actions::delete_optimize_session,
);

pub(in crate::routes::commands) async fn record_optimize_agent_event(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let session_id = i64_arg(args, "sessionId")?;
    let content = string_arg(args, "content")?;
    optimize_actions::record_optimize_agent_event(state.runtime(), user_id, session_id, content)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
