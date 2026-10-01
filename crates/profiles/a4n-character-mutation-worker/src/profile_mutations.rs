use super::*;

impl WorkerState {
    pub(super) async fn create(
        &self,
        request: WorkerEnvelope<CreateInput>,
    ) -> Result<CharacterProfileCreateResult, WorkerFailure> {
        validate_request(&request, CHARACTER_MUTATION_CREATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = a4n_db::character::create_profile(
            self.database.connection(),
            input.project_id,
            input.name.trim(),
            input.base_attributes,
        )
        .await
        .map_err(create_error)?;
        Ok(CharacterProfileCreateResult {
            profile_id: profile.id,
            warnings: Vec::new(),
        })
    }

    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateInput>,
    ) -> Result<CharacterProfileMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_MUTATION_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload.0;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = a4n_db::character::update_initial_profile(
            self.database.connection(),
            input.project_id,
            input.profile_id,
            input.expected_current_state,
            input.base_attributes,
        )
        .await
        .map_err(character_db_error)?;
        Ok(CharacterProfileMutationResult {
            profile: character_profile_summary(&profile)?,
            warnings: Vec::new(),
        })
    }

    pub(super) async fn delete_profile(
        &self,
        request: WorkerEnvelope<ProfileIdInput>,
    ) -> Result<CharacterProfileDeleteResult, WorkerFailure> {
        validate_request(&request, CHARACTER_MUTATION_DELETE_OPERATION)?;
        let user_id = user_id(&request)?;
        let profile =
            profile_for_editor(&self.database, user_id, request.payload.profile_id).await?;
        a4n_db::character::delete_profile_cascade_transaction(
            self.database.connection(),
            profile.id,
        )
        .await
        .map_err(character_profile_delete_error)?;
        Ok(CharacterProfileDeleteResult {
            warnings: Vec::new(),
        })
    }
}
