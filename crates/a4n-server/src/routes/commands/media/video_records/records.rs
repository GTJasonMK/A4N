use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, i64_arg, to_value, video_records_actions,
};

pub(in crate::routes::commands) async fn list_videos_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let videos = video_records_actions::list_videos_by_chapter(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(videos)
}

pub(in crate::routes::commands) async fn toggle_video_favorite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let updated = video_records_actions::toggle_video_favorite(state.runtime(), user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn update_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let input: video_records_actions::UpdateGeneratedVideoInput =
        command_input(args, "update_video")?;
    let updated = video_records_actions::update_video(state.runtime(), user_id, video_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn select_segment_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let video_id = i64_arg(args, "videoId")?;
    let input: a4n_api_contract::video::SelectSegmentVideoInput =
        command_input(args, "select_segment_video")?;
    let updated = video_records_actions::select_segment_video(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        video_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn delete_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    video_records_actions::delete_video(state.runtime(), user_id, video_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn delete_chapter_video_assets(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let result = video_records_actions::delete_chapter_video_assets(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn load_video_playback_data(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let playback =
        video_records_actions::load_video_playback_data(state.runtime(), user_id, video_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(playback)
}
