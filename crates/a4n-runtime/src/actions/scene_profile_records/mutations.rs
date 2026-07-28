use super::{SceneProfileInput, SceneProfileSummary, conflicts, draft, projections};
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

pub async fn create_scene_profile(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    parse_downstream_conflict_strategy(&input.downstream_strategy)?;
    let profile = state
        .scene_profile()
        .create_scene_profile_validated(project_id, draft::into_draft(input))
        .await
        .map_err(|error| error.to_string())?;
    projections::to_summary(state.scene_profile(), state.profile_material(), &profile).await
}

pub async fn update_scene_profile(
    state: &AppState,
    user_id: i64,
    scene_profile_id: i64,
    input: SceneProfileInput,
) -> Result<SceneProfileSummary, String> {
    let scene_profile = state.scene_profile();
    let project_id = conflicts::scene_profile_project_id(&scene_profile, scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream_strategy = parse_downstream_conflict_strategy(&input.downstream_strategy)?;
    let downstream = conflicts::resolve_scene_profile_downstream_conflict(
        state.profile_material(),
        project_id,
        scene_profile_id,
        downstream_strategy,
        "修改",
    )
    .await?;
    let profile = scene_profile
        .update_scene_profile_validated(scene_profile_id, draft::into_draft(input))
        .await
        .map_err(|error| error.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await?;
    projections::to_summary(scene_profile, state.profile_material(), &profile).await
}

pub async fn delete_scene_profile(
    state: &AppState,
    user_id: i64,
    scene_profile_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let scene_profile = state.scene_profile();
    let project_id = conflicts::scene_profile_project_id(&scene_profile, scene_profile_id).await?;
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let downstream = conflicts::resolve_scene_profile_downstream_conflict(
        state.profile_material(),
        project_id,
        scene_profile_id,
        downstream_strategy,
        "删除",
    )
    .await?;
    scene_profile
        .delete_scene_profile_records_validated(scene_profile_id)
        .await
        .map_err(|error| error.to_string())?;
    delete_downstream_images_if_requested(state, user_id, downstream).await
}
