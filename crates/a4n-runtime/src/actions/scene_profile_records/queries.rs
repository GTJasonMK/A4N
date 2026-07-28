use super::{SceneProfileSummary, projections};
use crate::{
    AppState,
    actions::{access, profile_materials},
};
use a4n_orchestrator::profile_material::PROFILE_KIND_SCENE;

pub async fn list_scene_profiles(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<SceneProfileSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let profiles = state
        .scene_profile()
        .scene_profiles_by_project(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let occurrence_counts =
        projections::occurrence_counts_by_profile(state.scene_profile(), project_id, &profile_ids)
            .await?;
    let image_counts = profile_materials::count_profile_material_images_by_profile(
        state.profile_material(),
        project_id,
        PROFILE_KIND_SCENE,
        &profile_ids,
    )
    .await?;
    Ok(profiles
        .iter()
        .map(|profile| {
            projections::to_summary_with_counts(
                profile,
                occurrence_counts.get(&profile.id).copied().unwrap_or(0),
                image_counts.get(&profile.id).copied().unwrap_or(0),
            )
        })
        .collect())
}
