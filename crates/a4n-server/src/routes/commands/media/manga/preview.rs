use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, i64_arg, manga_pipeline_actions, optional_string_field,
    required_json_i32, to_value,
};

pub(in crate::routes::commands) async fn preview_start_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "preview_start_manga_pipeline_request",
    )?;
    let start_from_stage = manga_pipeline_actions::parse_manga_pipeline_start_stage(
        optional_string_field(args, "startFromStage")?,
    )?;
    preview_manga_pipeline_request(
        state,
        user_id,
        project_id,
        chapter_number,
        false,
        start_from_stage,
    )
    .await
}

pub(in crate::routes::commands) async fn preview_resume_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "preview_resume_manga_pipeline_request",
    )?;
    preview_manga_pipeline_request(state, user_id, project_id, chapter_number, true, None).await
}

pub(in crate::routes::commands) async fn preview_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<manga_pipeline_actions::MangaPipelineStartStage>,
) -> Result<Value, ApiError> {
    to_value(
        manga_pipeline_actions::preview_manga_pipeline_requests(
            state.runtime(),
            user_id,
            project_id,
            chapter_number,
            resume_from_checkpoint,
            start_from_stage,
        )
        .await?,
    )
}
