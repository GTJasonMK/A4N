use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::character::{
    AddProfileAttributeInput, AttributeEventSummary, AttributeHistoryInput,
    ModifyProfileAttributeInput,
};

pub async fn add_profile_attribute(
    state: &AppState,
    user_id: i64,
    input: AddProfileAttributeInput,
) -> Result<(), String> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .character_profile_attribute_mutation_application()
        .add(user_id, &host_port, input)
        .await
        .map_err(app_error_message)
}

pub async fn modify_profile_attribute(
    state: &AppState,
    user_id: i64,
    input: ModifyProfileAttributeInput,
) -> Result<(), String> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .character_profile_attribute_mutation_application()
        .modify(user_id, &host_port, input)
        .await
        .map_err(app_error_message)
}

pub async fn attribute_history(
    state: &AppState,
    user_id: i64,
    input: AttributeHistoryInput,
) -> Result<Vec<AttributeEventSummary>, String> {
    state
        .character_timeline_read_application()
        .attribute_history(user_id, input)
        .await
        .map_err(app_error_message)
}
