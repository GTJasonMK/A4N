use super::*;

impl WorkerState {
    pub(super) async fn add_attribute(
        &self,
        request: WorkerEnvelope<AddProfileAttributeInput>,
    ) -> Result<AttributeEventMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_ADD_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::add_profile_attribute(
            self.database.connection(),
            CharacterAttributeValueChangeInput {
                profile_id: input.profile_id,
                chapter_number: input.chapter_number,
                category: input.category.as_str().to_string(),
                key: input.key,
                value: input.value,
                change_description: None,
                cause: input.cause,
                evidence: input.evidence,
            },
        )
        .await
        .map_err(character_db_error)?;
        Ok(AttributeEventMutationResult {
            project_id: profile.project_id,
            event_id: result.event.id,
        })
    }

    pub(super) async fn modify_attribute(
        &self,
        request: WorkerEnvelope<ModifyProfileAttributeInput>,
    ) -> Result<AttributeEventMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_MODIFY_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::modify_profile_attribute(
            self.database.connection(),
            CharacterAttributeValueChangeInput {
                profile_id: input.profile_id,
                chapter_number: input.chapter_number,
                category: input.category.as_str().to_string(),
                key: input.key,
                value: input.new_value,
                change_description: None,
                cause: input.cause,
                evidence: input.evidence,
            },
        )
        .await
        .map_err(character_db_error)?;
        Ok(AttributeEventMutationResult {
            project_id: profile.project_id,
            event_id: result.event.id,
        })
    }

    pub(super) async fn request_attribute_deletion(
        &self,
        request: WorkerEnvelope<RequestAttributeDeletionInput>,
    ) -> Result<DeletionMarkSummary, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_REQUEST_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let mark = a4n_db::character::request_attribute_deletion(
            self.database.connection(),
            CharacterAttributeDeletionRequestInput {
                profile_id: input.profile_id,
                category: input.category.as_str().to_string(),
                key: input.key,
                chapter_number: input.chapter_number,
                reason: input.reason,
                evidence: input.evidence,
            },
        )
        .await
        .map_err(character_db_error)?;
        deletion_mark_summary(&mark)
    }

    pub(super) async fn execute_attribute_deletion(
        &self,
        request: WorkerEnvelope<ExecuteAttributeDeletionInput>,
    ) -> Result<AttributeEventMutationResult, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_EXECUTE_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let profile = profile_for_editor(&self.database, user_id, input.profile_id).await?;
        let result = a4n_db::character::execute_ready_attribute_deletion(
            self.database.connection(),
            CharacterAttributeDeletionExecutionInput {
                profile_id: input.profile_id,
                category: input.category.as_str().to_string(),
                key: input.key,
                event_cause: input.cause,
                evidence: input.evidence,
                chapter_number: input.chapter_number,
            },
        )
        .await
        .map_err(character_db_error)?;
        Ok(AttributeEventMutationResult {
            project_id: profile.project_id,
            event_id: result.event.id,
        })
    }

    pub(super) async fn reset_attribute_deletion(
        &self,
        request: WorkerEnvelope<ResetDeletionMarksInput>,
    ) -> Result<u64, WorkerFailure> {
        validate_request(&request, CHARACTER_ATTRIBUTE_RESET_DELETION_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        profile_for_editor(&self.database, user_id, input.profile_id).await?;
        a4n_db::character::reset_deletion_marks_in(
            self.database.connection(),
            input.profile_id,
            input.category.as_str(),
            &input.key,
        )
        .await
        .map_err(character_db_error)
    }
}
