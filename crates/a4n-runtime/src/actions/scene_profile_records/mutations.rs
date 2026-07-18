use super::{SceneProfileInput, SceneProfileSummary, conflicts, projections};
use crate::{
    AppState,
    actions::{
        access,
        dependency_conflict::{DownstreamConflictStrategy, delete_downstream_images_if_requested},
    },
};
use a4n_orchestrator::scene_profile::normalize_scene_profile_record_input;

pub async fn create_scene_profile(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let record = normalize_scene_profile_record_input(input.into_draft())
        .map_err(|error| error.to_string())?;
    let profile = state
        .orchestrator
        .create_scene_profile_record(project_id, record)
        .await
        .map_err(|error| error.to_string())?;
    projections::to_summary(state, &profile).await
}

pub async fn update_scene_profile(
    state: &AppState,
    user_id: i64,
    scene_profile_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    let project_id = conflicts::scene_profile_project_id(state, scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_scene_profile_downstream_conflict(
        state,
        project_id,
        scene_profile_id,
        input.downstream_strategy,
        "修改",
    )
    .await?;
    let record = normalize_scene_profile_record_input(input.into_draft())
        .map_err(|error| error.to_string())?;
    let profile = state
        .orchestrator
        .update_scene_profile_record(scene_profile_id, record)
        .await
        .map_err(|error| error.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await?;
    projections::to_summary(state, &profile).await
}

pub async fn delete_scene_profile(
    state: &AppState,
    user_id: i64,
    scene_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let project_id = conflicts::scene_profile_project_id(state, scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_scene_profile_downstream_conflict(
        state,
        project_id,
        scene_profile_id,
        downstream_strategy,
        "删除",
    )
    .await?;
    state
        .orchestrator
        .delete_scene_profile_records(scene_profile_id)
        .await
        .map_err(|error| error.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await
}
