use a4n_core::models::character::{attribute_event, protagonist_profile};

use super::CharacterAttributeLifecycleError;
use crate::{OrchestratorContext, character::character_db_error};
use a4n_db::character::{
    AppliedCharacterImplicitDecisionRecord, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionExecutionResult, CharacterAttributeMutationResult,
    CharacterAttributeValueChangeInput, CharacterImplicitReviewDecisionRecordInput,
};

impl OrchestratorContext {
    pub async fn add_profile_attribute(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, CharacterAttributeLifecycleError> {
        let result = a4n_db::character::add_profile_attribute(self.db.connection(), input)
            .await
            .map_err(character_db_error)
            .map_err(CharacterAttributeLifecycleError::Persistence)?;
        self.sync_attribute_event_after_write(&result.profile, &result.event)
            .await?;
        Ok(result)
    }

    pub async fn modify_profile_attribute(
        &self,
        input: CharacterAttributeValueChangeInput,
    ) -> Result<CharacterAttributeMutationResult, CharacterAttributeLifecycleError> {
        let result = a4n_db::character::modify_profile_attribute(self.db.connection(), input)
            .await
            .map_err(character_db_error)
            .map_err(CharacterAttributeLifecycleError::Persistence)?;
        self.sync_attribute_event_after_write(&result.profile, &result.event)
            .await?;
        Ok(result)
    }

    pub async fn execute_ready_attribute_deletion(
        &self,
        input: CharacterAttributeDeletionExecutionInput,
    ) -> Result<CharacterAttributeDeletionExecutionResult, CharacterAttributeLifecycleError> {
        let result =
            a4n_db::character::execute_ready_attribute_deletion(self.db.connection(), input)
                .await
                .map_err(character_db_error)
                .map_err(CharacterAttributeLifecycleError::Persistence)?;
        self.sync_attribute_event_after_write(&result.profile, &result.event)
            .await?;
        Ok(result)
    }

    pub async fn apply_implicit_review_decision_record(
        &self,
        input: CharacterImplicitReviewDecisionRecordInput,
    ) -> Result<AppliedCharacterImplicitDecisionRecord, CharacterAttributeLifecycleError> {
        let applied =
            a4n_db::character::apply_implicit_review_decision_record(self.db.connection(), input)
                .await
                .map_err(character_db_error)
                .map_err(CharacterAttributeLifecycleError::Persistence)?;
        if let Some(event) = implicit_review_attribute_event(&applied) {
            self.sync_attribute_event_after_write(&applied.profile, event)
                .await?;
        }
        Ok(applied)
    }

    async fn sync_attribute_event_after_write(
        &self,
        profile: &protagonist_profile::Model,
        event: &attribute_event::Model,
    ) -> Result<(), CharacterAttributeLifecycleError> {
        self.sync_character_attribute_event_rag(profile.project_id, event, &profile.name)
            .await
            .map_err(CharacterAttributeLifecycleError::rag)
    }
}

pub(super) fn implicit_review_attribute_event(
    applied: &AppliedCharacterImplicitDecisionRecord,
) -> Option<&attribute_event::Model> {
    applied.event.as_ref()
}
