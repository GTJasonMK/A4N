use super::access::require_profile_for_user;
use super::contracts::{
    AddProfileAttributeInput, AttributeEventSummary, AttributeHistoryInput,
    AttributeMutationResultSummary, ModifyProfileAttributeInput,
};
use super::projections::attribute_event_summary;
use crate::AppState;
use crate::actions::app_error_message;
use a4n_orchestrator::character::{
    CharacterAttributeEventHistoryFilter, CharacterAttributeValueChangeInput,
};

pub async fn add_profile_attribute(
    state: &AppState,
    user_id: i64,
    input: AddProfileAttributeInput,
) -> Result<AttributeMutationResultSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let result = state
        .orchestrator()
        .add_profile_attribute_validated(CharacterAttributeValueChangeInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            category: input.category,
            key: input.key,
            value: input.value,
            change_description: None,
            cause: input.cause,
            evidence: input.evidence,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(AttributeMutationResultSummary {
        event: attribute_event_summary(&result.event),
    })
}

pub async fn modify_profile_attribute(
    state: &AppState,
    user_id: i64,
    input: ModifyProfileAttributeInput,
) -> Result<AttributeMutationResultSummary, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let result = state
        .orchestrator()
        .modify_profile_attribute_validated(CharacterAttributeValueChangeInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            category: input.category,
            key: input.key,
            value: input.new_value,
            change_description: None,
            cause: input.cause,
            evidence: input.evidence,
        })
        .await
        .map_err(|error| error.to_string())?;
    Ok(AttributeMutationResultSummary {
        event: attribute_event_summary(&result.event),
    })
}

pub async fn attribute_history(
    state: &AppState,
    user_id: i64,
    input: AttributeHistoryInput,
) -> Result<Vec<AttributeEventSummary>, String> {
    require_profile_for_user(state, user_id, input.profile_id).await?;
    let events = state
        .orchestrator()
        .attribute_history(
            input.profile_id,
            CharacterAttributeEventHistoryFilter {
                chapter_from: input.chapter_from,
                chapter_to: input.chapter_to,
                category: input.category,
                key: input.key,
                operation: input.operation,
                limit: input.limit,
            },
        )
        .await
        .map_err(app_error_message)?;
    Ok(events.iter().map(attribute_event_summary).collect())
}
