use super::super::reset::reset_generated_data;
use crate::AppState;
use crate::actions::access;
use a4n_api_contract::project::{
    CreateCanvasProjectInput, CreateDerivedProjectInput, CreateProjectInput,
    CreateStandaloneComicProjectInput, ProjectGeneratedDataResetResponse, ProjectSummary,
    UpdateProjectInput,
};

pub async fn create_project(
    state: &AppState,
    user_id: i64,
    input: CreateProjectInput,
) -> Result<ProjectSummary, String> {
    let project = state
        .project_mutation_application()
        .create(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(project)
}

pub async fn create_derived_project(
    state: &AppState,
    user_id: i64,
    input: CreateDerivedProjectInput,
) -> Result<ProjectSummary, String> {
    let project = state
        .project_mutation_application()
        .create_derived(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(project)
}

pub async fn create_canvas_project(
    state: &AppState,
    user_id: i64,
    input: CreateCanvasProjectInput,
) -> Result<ProjectSummary, String> {
    state
        .project_mutation_application()
        .create_canvas(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn create_standalone_comic_project(
    state: &AppState,
    user_id: i64,
    input: CreateStandaloneComicProjectInput,
) -> Result<ProjectSummary, String> {
    state
        .project_mutation_application()
        .create_standalone_comic(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn update_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateProjectInput,
) -> Result<ProjectSummary, String> {
    let project = state
        .project_mutation_application()
        .update(user_id, project_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(project)
}

pub async fn delete_project(state: &AppState, user_id: i64, project_id: i64) -> Result<(), String> {
    access::verify_project_owner_string(state, user_id, project_id).await?;
    state
        .destructive_lifecycle()
        .delete_project(state, user_id, project_id)
        .await?;
    if let Err(error) =
        crate::cleanup_outbox::drain_cleanup_outbox(state, Some(project_id), 1).await
    {
        tracing::warn!(project_id, error = %error, "项目已删除，提交后 cleanup 将由 outbox 重试");
    }
    Ok(())
}

pub async fn reset_project_generated_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectGeneratedDataResetResponse, String> {
    access::verify_project_owner_string(state, user_id, project_id).await?;
    reset_generated_data(state, user_id, project_id).await
}
