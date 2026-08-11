use super::{
    ObjectProfileDeleteResult, ObjectProfileInput, ObjectProfileMutationResult, conflicts, draft,
    projections,
};
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
) -> Result<ObjectProfileMutationResult, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    parse_downstream_conflict_strategy(&input.downstream_strategy)?;
    let profile = state
        .orchestrator()
        .create_object_profile_validated(project_id, draft::into_draft(input))
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
    object_profile_id: i64,
    input: ObjectProfileInput,
) -> Result<ObjectProfileMutationResult, String> {
    let asset = state.orchestrator();
    let project_id = conflicts::object_profile_project_id(&asset, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream_strategy = parse_downstream_conflict_strategy(&input.downstream_strategy)?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state.orchestrator(),
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
    object_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<ObjectProfileDeleteResult, String> {
    let asset = state.orchestrator();
    let project_id = conflicts::object_profile_project_id(&asset, object_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_object_profile_downstream_conflict(
        state.orchestrator(),
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
    Ok(ObjectProfileDeleteResult {
        warnings: downstream_cleanup_warnings(
            delete_downstream_images_if_requested(state, user_id, downstream).await,
        ),
    })
}

fn downstream_cleanup_warnings(result: Result<(), String>) -> Vec<String> {
    result
        .err()
        .map(|error| format!("下游图片清理失败：{error}"))
        .into_iter()
        .collect()
}
