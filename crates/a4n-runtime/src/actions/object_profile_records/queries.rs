use super::{ObjectProfileSummary, projections};
use crate::{
    AppState,
    actions::{access, profile_materials},
};
use a4n_domain::profile_material::ProfileMaterialKind;

pub async fn list_object_profiles(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ObjectProfileSummary>, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let profiles = state
        .orchestrator()
        .object_profiles_by_project(project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let image_counts = profile_materials::count_profile_material_images_by_profile(
        state.orchestrator(),
        project_id,
        ProfileMaterialKind::Object,
        &profile_ids,
    )
    .await?;
    Ok(profiles
        .iter()
        .map(|profile| {
            projections::to_summary_with_image_count(
                profile,
                image_counts.get(&profile.id).copied().unwrap_or(0),
            )
        })
        .collect())
}
