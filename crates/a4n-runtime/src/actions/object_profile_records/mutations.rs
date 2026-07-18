use super::{ObjectProfileInput, ObjectProfileSummary, conflicts, projections};
use crate::{
    AppState,
    actions::{
        access,
        dependency_conflict::{DownstreamConflictStrategy, delete_downstream_images_if_requested},
    },
};
use a4n_orchestrator::asset::normalize_object_profile_record_input;

pub async fn create_object_profile(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let record = normalize_object_profile_record_input(input.into_draft())
        .map_err(|error| error.to_string())?;
    let profile = state
        .orchestrator
        .create_object_profile_record(project_id, record)
        .await
        .map_err(crate::actions::app_error_message)?;
    projections::to_summary(state, &profile).await
}

pub async fn update_object_profile(
    state: &AppState,
    user_id: i64,
    object_profile_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    let project_id = conflicts::object_profile_project_id(state, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state,
        project_id,
        object_profile_id,
        input.downstream_strategy,
        "修改",
    )
    .await?;
    let record = normalize_object_profile_record_input(input.into_draft())
        .map_err(|error| error.to_string())?;
    let profile = state
        .orchestrator
        .update_object_profile_record(object_profile_id, record)
        .await
        .map_err(crate::actions::app_error_message)?;
    delete_downstream_images_if_requested(state, user_id, downstream).await?;
    projections::to_summary(state, &profile).await
}

pub async fn delete_object_profile(
    state: &AppState,
    user_id: i64,
    object_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let project_id = conflicts::object_profile_project_id(state, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state,
        project_id,
        object_profile_id,
        downstream_strategy,
        "删除",
    )
    .await?;
    state
        .orchestrator
        .delete_object_profile_record(object_profile_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    delete_downstream_images_if_requested(state, user_id, downstream).await
}
