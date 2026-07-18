use crate::{
    OrchestratorContext,
    character::{CharacterProfileDeletionSummary, CharacterTimelineDeletionSummary},
    rag::RagSyncError,
};

use super::{
    contracts::{CharacterProfileRagDeletionSummary, CharacterTimelineRagDeletionSummary},
    plan::CharacterRagDeletionPlan,
};

struct CharacterRagDeletionCounts {
    deleted_profile_rag: usize,
    deleted_state_rag: usize,
    deleted_attribute_event_rag: usize,
}

impl OrchestratorContext {
    pub(crate) async fn delete_character_profile_rag_records(
        &self,
        deletion: &CharacterProfileDeletionSummary,
    ) -> Result<CharacterProfileRagDeletionSummary, RagSyncError> {
        let counts = self
            .delete_character_rag_plan(CharacterRagDeletionPlan::for_profile(deletion))
            .await?;
        Ok(CharacterProfileRagDeletionSummary {
            deleted_profile_rag: counts.deleted_profile_rag,
            deleted_state_rag: counts.deleted_state_rag,
            deleted_attribute_event_rag: counts.deleted_attribute_event_rag,
        })
    }

    pub(crate) async fn delete_character_timeline_rag_records(
        &self,
        project_id: i64,
        deletion: &CharacterTimelineDeletionSummary,
    ) -> Result<CharacterTimelineRagDeletionSummary, RagSyncError> {
        let counts = self
            .delete_character_rag_plan(CharacterRagDeletionPlan::for_timeline(project_id, deletion))
            .await?;
        Ok(CharacterTimelineRagDeletionSummary {
            deleted_state_rag: counts.deleted_state_rag,
            deleted_attribute_event_rag: counts.deleted_attribute_event_rag,
        })
    }

    async fn delete_character_rag_plan(
        &self,
        plan: CharacterRagDeletionPlan<'_>,
    ) -> Result<CharacterRagDeletionCounts, RagSyncError> {
        let deleted_profile_rag = match plan.profile_id {
            Some(profile_id) => {
                self.delete_protagonist_rag_by_ids(plan.project_id, &[profile_id])
                    .await?
            }
            None => 0,
        };
        let deleted_state_rag = self
            .delete_character_state_rag_by_ids(plan.project_id, plan.state_index_ids)
            .await?;
        let deleted_attribute_event_rag = self
            .delete_character_attribute_event_rag_by_ids(plan.project_id, plan.attribute_event_ids)
            .await?;
        Ok(CharacterRagDeletionCounts {
            deleted_profile_rag,
            deleted_state_rag,
            deleted_attribute_event_rag,
        })
    }
}
