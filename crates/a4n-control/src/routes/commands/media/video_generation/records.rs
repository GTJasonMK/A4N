use serde_json::{Value, json};

use super::super::super::{ApiError, ServerState, i64_arg, video_generation_actions};

pub(in crate::routes::commands) async fn poll_video_job(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let updated = video_generation_actions::poll_video_job(state.runtime(), user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json!(updated))
}
