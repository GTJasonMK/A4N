use serde_json::Value;

use super::super::super::super::{
    ApiError, ServerState, f32_arg, i64_arg, to_value, video_records_actions,
};

pub(in crate::routes::commands) async fn trim_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let start_secs = f32_arg(args, "startSecs")?;
    let end_secs = f32_arg(args, "endSecs")?;
    let trimmed =
        video_records_actions::trim_video(state.runtime(), user_id, video_id, start_secs, end_secs)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(trimmed)
}
