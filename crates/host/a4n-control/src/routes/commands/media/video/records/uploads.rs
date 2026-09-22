use serde_json::Value;

use super::super::super::super::{
    ApiError, ServerState, command_input, i64_arg, string_arg, to_value, video_records_actions,
};

pub(in crate::routes::commands) async fn upload_chapter_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let input: video_records_actions::UploadChapterVideoInput =
        command_input(args, "upload_chapter_video")?;
    let video = video_records_actions::upload_chapter_video(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(in crate::routes::commands) async fn upload_reference_media_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let input: video_records_actions::UploadReferenceMediaVideoInput =
        command_input(args, "upload_reference_media_video")?;
    let video = video_records_actions::upload_reference_media_video(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}

pub(in crate::routes::commands) async fn upload_assembly_insert_video(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let workflow_mode = string_arg(args, "workflowMode")?;
    let input: video_records_actions::UploadAssemblyInsertVideoInput =
        command_input(args, "upload_assembly_insert_video")?;
    let video = video_records_actions::upload_assembly_insert_video(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
        workflow_mode,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(video)
}
