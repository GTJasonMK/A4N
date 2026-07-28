use super::super::contracts::{
    CreateDerivedProjectInput, CreateFreeformVideoProjectInput, CreateProjectInput,
    ProjectGeneratedDataResetSummary, ProjectSummary, UpdateProjectInput,
};
use super::super::reset::reset_generated_data;
use super::super::summaries::project_summary_with_family_for_user;
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_orchestrator::collaboration::ProjectAccessRole;
use a4n_orchestrator::project::{
    CreateDerivedProjectParams, CreateFreeformVideoProjectParams, DerivedChapterScope,
};
use a4n_orchestrator::project::{optional_text, required_optional_text, required_text};

pub async fn create_project(
    state: &AppState,
    user_id: i64,
    input: CreateProjectInput,
) -> Result<ProjectSummary, String> {
    let title = required_text(&input.title, "项目标题")?;
    let description = optional_text(input.description.as_deref());
    let project_type = required_optional_text(input.project_type.as_deref(), "项目类型")?;
    let project = state
        .project_mutation()
        .create_project_validated(
            user_id,
            title.to_string(),
            Some(description).filter(|value| !value.is_empty()),
            Some(project_type.to_string()),
        )
        .await
        .map_err(app_error_message)?;
    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn create_derived_project(
    state: &AppState,
    user_id: i64,
    input: CreateDerivedProjectInput,
) -> Result<ProjectSummary, String> {
    let title = required_text(&input.title, "项目标题")?;
    let description = required_optional_text(input.description.as_deref(), "项目简介")?;
    let root = state
        .collaboration()
        .collaboration_root_project(input.root_project_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .collaboration()
        .verify_project_permission(input.root_project_id, user_id, ProjectAccessRole::Editor)
        .await
        .map_err(|error| error.to_string())?;
    let project = state
        .project_mutation()
        .create_derived_project_validated(
            root.user_id,
            CreateDerivedProjectParams {
                root_project_id: input.root_project_id,
                derived_kind: input.derived_kind,
                title: title.to_string(),
                description: description.to_string(),
                chapter_scope: DerivedChapterScope {
                    start_chapter: input.start_chapter,
                    end_chapter: input.end_chapter,
                },
                creation_options_json: input.creation_options,
            },
        )
        .await
        .map_err(app_error_message)?;
    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn create_freeform_video_project(
    state: &AppState,
    user_id: i64,
    input: CreateFreeformVideoProjectInput,
) -> Result<ProjectSummary, String> {
    let title = required_text(&input.title, "项目标题")?;
    let description = optional_text(input.description.as_deref());
    let project = state
        .project_mutation()
        .create_freeform_video_project_validated(
            user_id,
            CreateFreeformVideoProjectParams {
                title: title.to_string(),
                description,
            },
        )
        .await
        .map_err(app_error_message)?;
    project_summary_with_family_for_user(state, user_id, &project).await
}

pub async fn update_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: UpdateProjectInput,
) -> Result<ProjectSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let project = state
        .project_mutation()
        .update_project_validated(project_id, input.title, input.description)
        .await
        .map_err(app_error_message)?;
    project_summary_with_family_for_user(state, user_id, &project).await
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
) -> Result<ProjectGeneratedDataResetSummary, String> {
    access::verify_project_owner_string(state, user_id, project_id).await?;
    reset_generated_data(state, user_id, project_id).await
}
