use super::{ObjectProfileInput, ObjectProfileSummary, conflicts, draft, projections};
use crate::{
    AppState,
    actions::{
        access,
        dependency_conflict::{
            DownstreamConflictStrategy, delete_downstream_images_if_requested,
            parse_downstream_conflict_strategy,
        },
    },
};

pub async fn create_object_profile(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    parse_downstream_conflict_strategy(&input.downstream_strategy)?;
    let profile = state
        .asset()
        .create_object_profile_validated(project_id, draft::into_draft(input))
        .await
        .map_err(crate::actions::app_error_message)?;
    projections::to_summary(state.profile_material(), &profile).await
}

pub async fn update_object_profile(
    state: &AppState,
    user_id: i64,
    object_profile_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileSummary, String> {
    let asset = state.asset();
    let project_id = conflicts::object_profile_project_id(&asset, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream_strategy = parse_downstream_conflict_strategy(&input.downstream_strategy)?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state.profile_material(),
        project_id,
        object_profile_id,
        downstream_strategy,
        "修改",
    )
    .await?;
    let profile = asset
        .update_object_profile_validated(object_profile_id, draft::into_draft(input))
        .await
        .map_err(crate::actions::app_error_message)?;
    delete_downstream_images_if_requested(state, user_id, downstream).await?;
    projections::to_summary(state.profile_material(), &profile).await
}

pub async fn delete_object_profile(
    state: &AppState,
    user_id: i64,
    object_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let asset = state.asset();
    let project_id = conflicts::object_profile_project_id(&asset, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state.profile_material(),
        project_id,
        object_profile_id,
        downstream_strategy,
        "删除",
    )
    .await?;
    asset
        .delete_object_profile_record_validated(object_profile_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    delete_downstream_images_if_requested(state, user_id, downstream).await
}
