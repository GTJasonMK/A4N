use super::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ObjectProfileDeleteResult,
    ObjectProfileMutationResult, UpdateObjectProfileInput, conflicts, projections,
};
use crate::{
    AppState,
    actions::{
        access,
        dependency_conflict::{delete_downstream_images_if_requested, downstream_cleanup_warnings},
    },
};

pub async fn create_object_profile(
    state: &AppState,
    user_id: i64,
    input: CreateObjectProfileInput,
) -> Result<ObjectProfileMutationResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = state
        .orchestrator()
        .create_object_profile_validated(input.project_id, input.profile)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(ObjectProfileMutationResult {
        profile: projections::to_summary(state.orchestrator(), &profile).await?,
        warnings: Vec::new(),
    })
}

pub async fn update_object_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateObjectProfileInput,
) -> Result<ObjectProfileMutationResult, String> {
    let asset = state.orchestrator();
    let project_id = conflicts::object_profile_project_id(asset, input.object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state.orchestrator(),
        project_id,
        input.object_profile_id,
        input.downstream_strategy,
        "修改",
    )
    .await?;
    let profile = asset
        .update_object_profile_validated(input.object_profile_id, input.profile)
        .await
        .map_err(crate::actions::app_error_message)?;
    let warnings = downstream_cleanup_warnings(
        delete_downstream_images_if_requested(state, user_id, downstream).await,
    );
    Ok(ObjectProfileMutationResult {
        profile: projections::to_summary(state.orchestrator(), &profile).await?,
        warnings,
    })
}

pub async fn delete_object_profile(
    state: &AppState,
    user_id: i64,
    input: DeleteObjectProfileInput,
) -> Result<ObjectProfileDeleteResult, String> {
    let asset = state.orchestrator();
    let project_id = conflicts::object_profile_project_id(asset, input.object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state.orchestrator(),
        project_id,
        input.object_profile_id,
        input.downstream_strategy,
        "删除",
    )
    .await?;
    asset
        .delete_object_profile_record_validated(input.object_profile_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(ObjectProfileDeleteResult {
        warnings: downstream_cleanup_warnings(
            delete_downstream_images_if_requested(state, user_id, downstream).await,
        ),
    })
}
