use a4n_api_contract::manga::{MangaChapterInput, MangaPipelineStatusesInput};
use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, command_input, manga_pipeline_actions, manga_records_actions, to_value,
};

pub(in crate::routes::commands) async fn prepare_standalone_comic_source(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::PrepareStandaloneComicSourceInput =
        command_input(args, "prepare_standalone_comic_source")?;
    let source =
        manga_pipeline_actions::prepare_standalone_comic_source(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(source)
}

pub(in crate::routes::commands) async fn get_manga_pipeline_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: MangaChapterInput = command_input(args, "get_manga_pipeline_status")?;
    let status = manga_records_actions::get_manga_pipeline_status(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(status)
}

pub(in crate::routes::commands) async fn list_manga_pipeline_statuses(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: MangaPipelineStatusesInput = command_input(args, "list_manga_pipeline_statuses")?;
    let statuses =
        manga_records_actions::list_manga_pipeline_statuses(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(statuses)
}

pub(in crate::routes::commands) async fn cancel_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: MangaChapterInput = command_input(args, "cancel_manga_pipeline")?;
    let cancelled = manga_records_actions::cancel_manga_pipeline(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json!(cancelled))
}

pub(in crate::routes::commands) async fn start_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::MangaPipelineStartInput =
        command_input(args, "start_manga_pipeline")?;
    let input = manga_pipeline_actions::MangaPipelineStartInput {
        meta: input.meta,
        project_id: input.project_id,
        chapter_number: input.chapter_number,
        page_number: input.page_number,
        stop_after: input.stop_after,
        image_size: input.image_size,
        resume_from_checkpoint: false,
    };
    start_or_resume_manga_pipeline(state, user_id, input).await
}

pub(in crate::routes::commands) async fn resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::MangaPipelineResumeInput =
        command_input(args, "resume_manga_pipeline")?;
    let input = manga_pipeline_actions::MangaPipelineStartInput {
        meta: input.meta,
        project_id: input.project_id,
        chapter_number: input.chapter_number,
        page_number: input.page_number,
        stop_after: input.stop_after,
        image_size: input.image_size,
        resume_from_checkpoint: true,
    };
    start_or_resume_manga_pipeline(state, user_id, input).await
}

pub(in crate::routes::commands) async fn start_or_resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    input: manga_pipeline_actions::MangaPipelineStartInput,
) -> Result<Value, ApiError> {
    manga_pipeline_actions::preflight_manga_pipeline(
        state.runtime(),
        user_id,
        input.project_id,
        input.chapter_number,
        input.page_number,
        input.stop_after,
    )
    .await?;
    let handle =
        manga_pipeline_actions::start_manga_pipeline_typed(state.runtime(), user_id, input)
            .await
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
    to_value(handle)
}
