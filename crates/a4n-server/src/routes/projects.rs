use super::auth::authenticated_user;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_core::models::project::novel_project;
use a4n_runtime::actions::project_records as project_records_actions;
use a4n_runtime::actions::project_records::{
    CreateDerivedProjectInput, CreateFreeformVideoProjectInput, CreateProjectInput,
    PaginatedProjects, ProjectSummary, SourceSnapshotChapterSummary,
};
use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ListProjectsQuery {
    page: Option<u64>,
    page_size: Option<u64>,
    project_kind: Option<String>,
    root_project_id: Option<i64>,
    access_scope: Option<String>,
}

pub(super) async fn list_projects(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<ListProjectsQuery>,
) -> Result<Json<PaginatedProjects>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let page = positive_or_default(query.page, 1, "page")?;
    let page_size = positive_or_default(query.page_size, 20, "page_size")?;
    let projects = project_records_actions::list_projects(
        &state,
        user.id,
        page,
        page_size,
        query.project_kind.as_deref(),
        query.root_project_id,
        query.access_scope.as_deref(),
    )
    .await
    .map_err(ApiError::bad_request)?;

    Ok(Json(projects))
}

pub async fn create_project(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<CreateProjectInput>,
) -> Result<Json<ProjectSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let project = project_records_actions::create_project(&state, user.id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(project))
}

pub async fn create_derived_project(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<CreateDerivedProjectInput>,
) -> Result<Json<ProjectSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let project = project_records_actions::create_derived_project(&state, user.id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(project))
}

pub async fn create_freeform_video_project(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<CreateFreeformVideoProjectInput>,
) -> Result<Json<ProjectSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let project = project_records_actions::create_freeform_video_project(&state, user.id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(project))
}

pub async fn get_project(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<i64>,
) -> Result<Json<ProjectSummary>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let project = project_records_actions::get_project_summary(&state, user.id, project_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(project))
}

pub async fn list_source_snapshot_chapters(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<i64>,
) -> Result<Json<Vec<SourceSnapshotChapterSummary>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let chapters =
        project_records_actions::list_source_snapshot_chapters(&state, user.id, project_id)
            .await
            .map_err(ApiError::bad_request)?;
    Ok(Json(chapters))
}

fn positive_or_default(value: Option<u64>, default: u64, field: &str) -> Result<u64, ApiError> {
    match value {
        Some(value) if value > 0 => Ok(value),
        Some(_) => Err(ApiError::bad_request(format!("{field} 必须为正整数"))),
        None => Ok(default),
    }
}

pub(crate) async fn project_summary_with_family_for_user(
    state: &ServerState,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<ProjectSummary, ApiError> {
    project_records_actions::project_summary_with_family_for_user(state, user_id, project)
        .await
        .map_err(ApiError::bad_request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_requests_reject_unknown_fields() {
        let create_err = serde_json::from_value::<CreateProjectInput>(json!({
            "title": "测试项目",
            "description": "简介",
            "project_type": "novel_long",
            "extra": true
        }))
        .expect_err("create project request must reject unknown fields");
        assert!(create_err.to_string().contains("unknown field"));

        let derived_err = serde_json::from_value::<CreateDerivedProjectInput>(json!({
            "root_project_id": 1,
            "derived_kind": "video",
            "title": "视频项目",
            "description": "简介",
            "start_chapter": 1,
            "end_chapter": 3,
            "creation_options": {},
            "extra": true
        }))
        .expect_err("derived project request must reject unknown fields");
        assert!(derived_err.to_string().contains("unknown field"));

        let freeform_err = serde_json::from_value::<CreateFreeformVideoProjectInput>(json!({
            "title": "视频画布项目",
            "description": "简介",
            "project_type": "novel_long"
        }))
        .expect_err("freeform video project request must reject project_type");
        assert!(freeform_err.to_string().contains("unknown field"));
    }

    #[test]
    fn project_list_query_rejects_unknown_fields() {
        let err = serde_json::from_value::<ListProjectsQuery>(json!({
            "page": 1,
            "page_size": 20,
            "project_kind": "text",
            "root_project_id": null,
            "recent": true
        }))
        .expect_err("project list query must reject unknown fields");
        assert!(err.to_string().contains("unknown field"));
    }
}
