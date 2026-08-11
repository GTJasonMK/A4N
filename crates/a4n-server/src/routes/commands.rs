mod admin;
mod canvas;
mod characters;
mod front;
mod galgame;
mod import_project;
mod input;
mod media;
mod request;
mod source_sync;
mod task;

mod dispatch {
    mod assets_and_media;
    mod config_and_rag;
    mod galgame;
    mod manga;
    mod projects_and_content;
    mod system_and_collaboration;
    mod video;

    use super::*;

    pub(super) async fn dispatch(
        state: &ServerState,
        user_id: i64,
        args: &Value,
        cmd: &str,
    ) -> Result<Value, ApiError> {
        if let Some(result) =
            Box::pin(config_and_rag::try_dispatch(state, user_id, args, cmd)).await?
        {
            return Ok(result);
        }
        if let Some(result) = Box::pin(projects_and_content::try_dispatch(
            state, user_id, args, cmd,
        ))
        .await?
        {
            return Ok(result);
        }
        if let Some(result) =
            Box::pin(assets_and_media::try_dispatch(state, user_id, args, cmd)).await?
        {
            return Ok(result);
        }
        if let Some(result) = Box::pin(video::try_dispatch(state, user_id, args, cmd)).await? {
            return Ok(result);
        }
        if let Some(result) = Box::pin(manga::try_dispatch(state, user_id, args, cmd)).await? {
            return Ok(result);
        }
        if let Some(result) = Box::pin(galgame::try_dispatch(state, user_id, args, cmd)).await? {
            return Ok(result);
        }
        if let Some(result) = Box::pin(system_and_collaboration::try_dispatch(
            state, user_id, args, cmd,
        ))
        .await?
        {
            return Ok(result);
        }
        Err(ApiError::bad_request(format!(
            "Web command 尚未实现: {cmd}"
        )))
    }
}

use self::canvas::*;
use self::characters::*;
use self::front::*;
use self::galgame::*;
use self::import_project::*;
use self::input::*;
use self::media::*;
use self::request::*;
use self::source_sync::*;
use self::task::*;

use super::auth::{authenticated_user, issue_auth_response, user_agent};
use super::error::ApiError;
use crate::state::ServerState;
use a4n_core::event::channel::TaskKind;
use a4n_runtime::host_api::auth_records as auth_records_actions;
use a4n_runtime::host_api::blueprint_records as blueprint_records_actions;
use a4n_runtime::host_api::chapter_content as chapter_content_actions;
use a4n_runtime::host_api::chapter_generation as chapter_generation_actions;
use a4n_runtime::host_api::chapter_scene as chapter_scene_actions;
use a4n_runtime::host_api::character_ai as character_ai_actions;
use a4n_runtime::host_api::character_conflict as character_conflict_actions;
use a4n_runtime::host_api::character_implicit_review as character_implicit_review_actions;
use a4n_runtime::host_api::character_records as character_records_actions;
use a4n_runtime::host_api::config_records as config_records_actions;
use a4n_runtime::host_api::embedding_model as embedding_model_actions;
use a4n_runtime::host_api::export_files as export_files_actions;
use a4n_runtime::host_api::foreshadowing_records as foreshadowing_records_actions;
use a4n_runtime::host_api::generation_analytics as generation_analytics_actions;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use a4n_runtime::host_api::image_records as image_records_actions;
use a4n_runtime::host_api::import_analysis as import_analysis_actions;
use a4n_runtime::host_api::import_file as import_file_actions;
use a4n_runtime::host_api::import_reverse as import_reverse_actions;
use a4n_runtime::host_api::import_review as import_review_actions;
use a4n_runtime::host_api::inspiration_records as inspiration_records_actions;
use a4n_runtime::host_api::logging as logging_actions;
use a4n_runtime::host_api::manga_pipeline as manga_pipeline_actions;
use a4n_runtime::host_api::manga_records as manga_records_actions;
use a4n_runtime::host_api::oauth as oauth_actions;
use a4n_runtime::host_api::object_profile_records as object_profile_records_actions;
use a4n_runtime::host_api::optimize as optimize_actions;
use a4n_runtime::host_api::outline_records as outline_records_actions;
use a4n_runtime::host_api::portrait_generation as portrait_generation_actions;
use a4n_runtime::host_api::portrait_records as portrait_records_actions;
use a4n_runtime::host_api::profile_materials as profile_materials_actions;
use a4n_runtime::host_api::project_collaboration as project_collaboration_actions;
use a4n_runtime::host_api::project_records as project_records_actions;
use a4n_runtime::host_api::project_status as project_status_actions;
use a4n_runtime::host_api::project_transfer as project_transfer_actions;
use a4n_runtime::host_api::prompt_records as prompt_records_actions;
use a4n_runtime::host_api::rag_search as rag_search_actions;
use a4n_runtime::host_api::scene_profile_records as scene_profile_records_actions;
use a4n_runtime::host_api::scene_profile_sync as scene_profile_sync_actions;
use a4n_runtime::host_api::source_sync as source_sync_actions;
use a4n_runtime::host_api::storage_paths as storage_paths_actions;
use a4n_runtime::host_api::task_records as task_records_actions;
use a4n_runtime::host_api::theme as theme_actions;
use a4n_runtime::host_api::video_generation as video_generation_actions;
use a4n_runtime::host_api::video_prompts as video_prompt_actions;
use a4n_runtime::host_api::video_records as video_records_actions;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

pub async fn invoke(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(cmd): Path<String>,
    Json(args): Json<Value>,
) -> Result<Response, ApiError> {
    let public_result = match cmd.as_str() {
        "login_user" => Some(login_user(&state, &headers, &args).await?),
        "register_user" => Some(register_user(&state, &headers, &args).await?),
        "refresh_token" => Some(refresh_token(&state, &args).await?),
        "oauth2_authorize_url" => Some(oauth2_authorize_url(&state, &args).await?),
        "oauth2_callback" => Some(oauth2_callback(&state, &headers, &args).await?),
        _ => None,
    };
    if let Some(result) = public_result {
        return Ok(Json(result).into_response());
    }

    let user = authenticated_user(&state, &headers).await?;
    let result = dispatch_runtime_command_inner(state.runtime(), user.id, args, cmd).await?;

    Ok(Json(result).into_response())
}

pub async fn dispatch_runtime_command(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    args: Value,
    cmd: String,
) -> Result<Value, String> {
    dispatch_runtime_command_inner(runtime, user_id, args, cmd)
        .await
        .map_err(|error| error.to_string())
}

async fn dispatch_runtime_command_inner(
    runtime: &a4n_runtime::AppState,
    user_id: i64,
    args: Value,
    cmd: String,
) -> Result<Value, ApiError> {
    let runtime_config_snapshot =
        a4n_runtime::runtime_config::snapshot_from_args(&args).map_err(ApiError::bad_request)?;
    let state = ServerState::from_runtime(runtime.clone());
    run_with_runtime_context(user_id, runtime_config_snapshot, async move {
        if let Some(command) = reverse_engineer_web_command(&cmd) {
            return run_reverse_engineer_web_command(&state, user_id, &args, command).await;
        }
        if is_canvas_web_command(&cmd) {
            return run_canvas_web_command(&state, user_id, &args, &cmd).await;
        }
        dispatch::dispatch(&state, user_id, &args, &cmd).await
    })
    .await
}
