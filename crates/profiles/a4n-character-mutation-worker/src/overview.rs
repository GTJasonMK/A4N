use super::*;

impl WorkerState {
    pub(super) async fn profile_project(
        &self,
        request: WorkerEnvelope<ProfileIdInput>,
    ) -> Result<i64, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_PROJECT_OPERATION)?;
        let user_id = user_id(&request)?;
        let profile_id = positive(request.payload.profile_id, "profile_id")?;
        let profile =
            a4n_db::character::profile_record_by_id(self.database.connection(), profile_id)
                .await
                .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
                .ok_or_else(|| WorkerFailure::not_found(format!("角色档案不存在: {profile_id}")))?;
        verify_project_editor(&self.database, profile.project_id, user_id).await?;
        Ok(profile.project_id)
    }

    pub(super) async fn list_profiles(
        &self,
        request: WorkerEnvelope<ListProfilesInput>,
    ) -> Result<Vec<CharacterProfileSummary>, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let project_id = positive(request.payload.project_id, "project_id")?;
        verify_project_editor(&self.database, project_id, user_id).await?;
        a4n_db::character::profiles_with_timeline_state_records(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(character_db_error)?
        .into_iter()
        .map(|(profile, timeline_started, editable)| {
            character_profile_summary_with_state(&profile, timeline_started, editable)
        })
        .collect()
    }

    pub(super) async fn character_arc(
        &self,
        request: WorkerEnvelope<NamedProfileInput>,
    ) -> Result<Option<Value>, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_ARC_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = named_profile(&self.database, input.project_id, &input.name).await?;
        Ok(profile.character_arc)
    }

    pub(super) async fn character_voice(
        &self,
        request: WorkerEnvelope<NamedProfileInput>,
    ) -> Result<Option<Value>, WorkerFailure> {
        validate_request(&request, CHARACTER_OVERVIEW_VOICE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        verify_project_editor(&self.database, input.project_id, user_id).await?;
        let profile = named_profile(&self.database, input.project_id, &input.name).await?;
        Ok(profile.voice_fingerprint)
    }
}
