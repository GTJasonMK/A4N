use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, i64_arg, string_arg, to_value, video_records_actions,
};

pub(in crate::routes::commands) async fn assemble_chapter_videos(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let preview = video_records_actions::assemble_chapter_videos(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn load_cached_assembled_chapter_video_preview(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let preview = video_records_actions::load_cached_assembled_chapter_video_preview(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn push_assembled_chapter_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let input: video_records_actions::PushAssembledChapterVideoInput =
        command_input(args, "push_assembled_chapter_video")?;
    let result = video_records_actions::push_assembled_chapter_video(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn update_assembly_push_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let video_id = i64_arg(args, "videoId")?;
    let input: video_records_actions::UpdateAssemblyPushStatusInput =
        command_input(args, "update_assembly_push_status")?;
    let video = video_records_actions::update_assembly_push_status(
        state.runtime(),
        user_id,
        video_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(in crate::routes::commands) async fn apply_assembly_postprocess_edit(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let push_video_id = i64_arg(args, "pushVideoId")?;
    let input: video_records_actions::ApplyAssemblyPostprocessEditInput =
        command_input(args, "apply_assembly_postprocess_edit")?;
    let video = video_records_actions::apply_assembly_postprocess_edit(
        state.runtime(),
        user_id,
        push_video_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}
