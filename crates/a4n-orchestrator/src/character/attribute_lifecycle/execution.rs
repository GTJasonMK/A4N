use a4n_core::models::character::{attribute_event, protagonist_profile};

use super::CharacterAttributeLifecycleError;
use crate::{character::character_db_error};
use a4n_db::character::{
    AppliedCharacterImplicitDecisionRecord, CharacterAttributeDeletionExecutionInput,
    CharacterAttributeDeletionExecutionResult, CharacterAttributeMutationResult,
    CharacterAttributeValueChangeInput, CharacterImplicitReviewDecisionRecordInput,
};

pub(super) fn implicit_review_attribute_event(
    applied: &AppliedCharacterImplicitDecisionRecord,
) -> Option<&attribute_event::Model> {
    applied.event.as_ref()
}
