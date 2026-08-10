use serde_json::Value;

use super::super::super::{ApiError, ServerState, command_input, manga_pipeline_actions, to_value};

pub(in crate::routes::commands) async fn preview_start_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::MangaPipelineStartInput =
        command_input(args, "preview_start_manga_pipeline_request")?;
    preview_manga_pipeline_request(
        state,
        user_id,
        input.project_id,
        input.chapter_number,
        false,
        input.start_from_stage,
    )
    .await
}

pub(in crate::routes::commands) async fn preview_resume_manga_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::MangaPipelineResumeInput =
        command_input(args, "preview_resume_manga_pipeline_request")?;
    preview_manga_pipeline_request(
        state,
        user_id,
        input.project_id,
        input.chapter_number,
        true,
        None,
    )
    .await
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
