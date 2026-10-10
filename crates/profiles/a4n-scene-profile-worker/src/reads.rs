use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<SceneProfileProjectInput>,
    ) -> Result<Vec<SceneProfileSummary>, WorkerFailure> {
        validate_request(&request, SCENE_PROFILE_LIST_OPERATION)?;
        let project_id = request.payload.project_id;
        request.require_project_scope(project_id)?;
        let user_id = request.principal.require_user_id()?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_error)?;
        let profiles = a4n_db::scene_profile::scene_profiles_by_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_scene_profile_db_error)?;
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        let occurrence_counts = a4n_db::scene_profile::scene_occurrence_counts_by_profile(
            self.database.connection(),
            project_id,
            &profile_ids,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_scene_profile_db_error)?;
        let image_counts = a4n_db::profile_material::profile_material_image_counts_by_profile(
            self.database.connection(),
            project_id,
            IMAGE_TYPE_SCENE_MATERIAL,
            &profile_ids,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_profile_material_db_error)?;
        Ok(profiles
            .iter()
            .map(|profile| {
                scene_profile_summary(
                    profile,
                    occurrence_counts
                        .get(&profile.id)
                        .copied()
                        .unwrap_or_default(),
                    image_counts.get(&profile.id).copied().unwrap_or_default(),
                )
            })
            .collect())
    }
}
