use super::auth::authenticated_user;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_runtime::actions::source_sync;
use axum::Json;
use axum::extract::State;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreviewSourceSyncRequest {
    derived_project_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DetachDerivedProjectRequest {
    derived_project_id: i64,
}

pub(super) async fn preview_source_sync(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<PreviewSourceSyncRequest>,
) -> Result<Json<source_sync::SourceSyncPlanSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let plan = source_sync::preview_source_sync(&state, user.id, input.derived_project_id).await?;
    Ok(Json(plan))
}

pub(super) async fn apply_source_sync_decisions(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<source_sync::ApplySourceSyncInput>,
) -> Result<Json<source_sync::SourceSyncRunSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let run = source_sync::apply_source_sync_decisions(&state, user.id, input).await?;
    Ok(Json(run))
}

pub(super) async fn detach_derived_project(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<DetachDerivedProjectRequest>,
) -> Result<Json<a4n_core::models::project::novel_project::Model>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let project =
        source_sync::detach_derived_project(&state, user.id, input.derived_project_id).await?;
    Ok(Json(project))
}
