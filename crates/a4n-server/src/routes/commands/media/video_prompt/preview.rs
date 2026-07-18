use serde_json::Value;

use super::super::super::{ApiError, ServerState, to_value, video_prompt_actions};
use super::input::video_prompt_input_from_args;

pub(in crate::routes::commands) async fn preview_chapter_video_frame_prompt_requests(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input =
        video_prompt_input_from_args(user_id, args, "preview_chapter_video_frame_prompt_requests")?;
    let preview = video_prompt_actions::preview_chapter_video_frame_prompt_model_request(
        state.runtime(),
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_chapter_video_prompt_requests(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input =
        video_prompt_input_from_args(user_id, args, "preview_chapter_video_prompt_requests")?;
    let preview =
        video_prompt_actions::preview_chapter_video_prompt_model_request(state.runtime(), input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}
