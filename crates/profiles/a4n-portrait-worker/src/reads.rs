use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<PortraitProjectInput>,
    ) -> Result<Vec<PortraitSummary>, WorkerFailure> {
        validate_request(&request, PORTRAIT_LIST_OPERATION)?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_permission(
            self,
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        a4n_db::character::character_portrait_records_by_project_records(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_database_error)?
        .iter()
        .map(portrait_summary)
        .collect()
    }

    pub(super) async fn primary(
        &self,
        request: WorkerEnvelope<PortraitProfileInput>,
    ) -> Result<PortraitLookupResponse, WorkerFailure> {
        validate_request(&request, PORTRAIT_PRIMARY_OPERATION)?;
        let profile_id =
            WorkerFailure::require_positive_id(request.payload.profile_id, "profile_id")?;
        let project_id =
            a4n_db::project::protagonist_profile_project_id(self.database.connection(), profile_id)
                .await
                .map_err(a4n_worker_db_adapter::map_project_family_db_error)?;
        let user_id = request.principal.require_user_id()?;
        verify_permission(
            self,
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        let portrait = a4n_db::character::primary_character_portrait_record_value(
            self.database.connection(),
            profile_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_database_error)?;
        Ok(PortraitLookupResponse {
            portrait: portrait.as_ref().map(portrait_summary).transpose()?,
        })
    }

    pub(super) async fn by_id(
        &self,
        request: WorkerEnvelope<PortraitRecordInput>,
    ) -> Result<PortraitLookupResponse, WorkerFailure> {
        validate_request(&request, PORTRAIT_BY_ID_OPERATION)?;
        let portrait_id =
            WorkerFailure::require_positive_id(request.payload.portrait_id, "portrait_id")?;
        let Some(portrait) = a4n_db::character::character_portrait_record_by_id(
            self.database.connection(),
            portrait_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_database_error)?
        else {
            return Ok(PortraitLookupResponse { portrait: None });
        };
        let user_id = request.principal.require_user_id()?;
        verify_permission(
            self,
            portrait.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(PortraitLookupResponse {
            portrait: Some(portrait_summary(&portrait)?),
        })
    }

    pub(super) async fn missing_seeds(
        &self,
        request: WorkerEnvelope<PortraitProjectInput>,
    ) -> Result<Vec<PortraitGenerationSeedWire>, WorkerFailure> {
        validate_request(&request, PORTRAIT_MISSING_SEEDS_OPERATION)?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        verify_permission(
            self,
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::character::missing_portrait_profile_records(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_database_error)?
            .iter()
            .map(|profile| {
                profile_portrait_seed_prompt(&portrait_profile(profile))
                    .map(|prompt| PortraitGenerationSeedWire {
                        profile_name: profile.name.clone(),
                        prompt,
                    })
                    .map_err(WorkerFailure::invalid_request)
            })
            .collect()
    }
}
