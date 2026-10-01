use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ObjectProfileProjectInput>,
    ) -> Result<Vec<ObjectProfileSummary>, WorkerFailure> {
        validate_request(&request, OBJECT_PROFILE_LIST_OPERATION)?;
        let project_id = request.payload.project_id;
        if request.project_id != Some(project_id) || project_id <= 0 {
            return Err(WorkerFailure::invalid_request(
                "worker scope project_id 与 payload 不一致或无效",
            ));
        }
        let user_id = user_id(&request)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_error)?;
        let profiles =
            a4n_db::asset::object_profiles_by_project(self.database.connection(), project_id)
                .await
                .map_err(asset_error)?;
        let profile_ids = profiles
            .iter()
            .map(|profile| profile.id)
            .collect::<Vec<_>>();
        let image_counts = a4n_db::profile_material::profile_material_image_counts_by_profile(
            self.database.connection(),
            project_id,
            IMAGE_TYPE_OBJECT_MATERIAL,
            &profile_ids,
        )
        .await
        .map_err(profile_material_error)?;
        Ok(profiles
            .iter()
            .map(|profile| {
                object_profile_summary(profile, image_counts.get(&profile.id).copied().unwrap_or(0))
            })
            .collect())
    }
}
