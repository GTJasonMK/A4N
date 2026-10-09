use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPageInput, DeleteMangaPipelineArtifactInput,
    DeleteMangaPromptInput, MangaChapterInput, OptimizeMangaPromptInput,
    PreviewMangaPipelineArtifactDeletionInput, SaveMangaPipelineArtifactInput,
    UpdateMangaPromptInput,
};
use serde_json::Value;

use super::super::super::{ApiError, ServerState, command_input, manga_records_actions, to_value};

pub(in crate::routes::commands) async fn list_manga_page_prompts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: MangaChapterInput = command_input(args, "list_manga_page_prompts")?;
    let page_prompts =
        manga_records_actions::list_manga_page_prompts(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(page_prompts)
}

pub(in crate::routes::commands) async fn list_manga_pipeline_artifacts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: MangaChapterInput = command_input(args, "list_manga_pipeline_artifacts")?;
    let artifacts =
        manga_records_actions::list_manga_pipeline_artifacts(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(artifacts)
}

pub(in crate::routes::commands) async fn save_manga_pipeline_artifact(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: SaveMangaPipelineArtifactInput =
        command_input(args, "save_manga_pipeline_artifact")?;
    let artifact =
        manga_records_actions::save_manga_pipeline_artifact(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(artifact)
}

pub(in crate::routes::commands) async fn preview_delete_manga_pipeline_artifact(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: PreviewMangaPipelineArtifactDeletionInput =
        command_input(args, "preview_delete_manga_pipeline_artifact")?;
    let preview = manga_records_actions::preview_delete_manga_pipeline_artifact(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn delete_manga_pipeline_artifact(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: DeleteMangaPipelineArtifactInput =
        command_input(args, "delete_manga_pipeline_artifact")?;
    manga_records_actions::delete_manga_pipeline_artifact(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn bind_manga_reference_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: BindMangaReferenceImageInput = command_input(args, "bind_manga_reference_image")?;
    let updated =
        manga_records_actions::bind_manga_reference_image(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn update_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: UpdateMangaPromptInput = command_input(args, "update_manga_prompt")?;
    let updated = manga_records_actions::update_manga_prompt(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn optimize_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: OptimizeMangaPromptInput = command_input(args, "optimize_manga_prompt")?;
    let updated = manga_records_actions::optimize_manga_prompt(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn delete_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: DeleteMangaPromptInput = command_input(args, "delete_manga_prompt")?;
    manga_records_actions::delete_manga_prompt(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn delete_manga_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: MangaChapterInput = command_input(args, "delete_manga_by_chapter")?;
    manga_records_actions::delete_manga_by_chapter(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn delete_manga_page(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: DeleteMangaPageInput = command_input(args, "delete_manga_page")?;
    manga_records_actions::delete_manga_page(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
