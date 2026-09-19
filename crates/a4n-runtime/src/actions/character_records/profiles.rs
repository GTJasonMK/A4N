use crate::AppState;
use crate::actions::app_error_message;
use a4n_api_contract::character::{
    CharacterProfileCreateResult, CharacterProfileDeleteResult, CharacterProfileMutationResult,
    CharacterProfileSummary, CreateProfileInput, ListProfilesInput, UpdateCharacterProfileInput,
};
use serde_json::Value;

pub async fn create_profile(
    state: &AppState,
    user_id: i64,
    input: CreateProfileInput,
) -> Result<CharacterProfileCreateResult, String> {
    let project_id = input.project_id;
    let mut result = state
        .character_profile_mutation_application()
        .create(user_id, input)
        .await
        .map_err(app_error_message)?;
    result
        .warnings
        .extend(profile_rag_sync_warnings(state, project_id).await);
    Ok(result)
}

pub async fn update_character_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateCharacterProfileInput,
) -> Result<CharacterProfileMutationResult, String> {
    let mut result = state
        .character_profile_mutation_application()
        .update(user_id, input)
        .await
        .map_err(app_error_message)?;
    result
        .warnings
        .extend(profile_rag_sync_warnings(state, result.profile.project_id).await);
    Ok(result)
}

pub async fn list_profiles(
    state: &AppState,
    user_id: i64,
    input: ListProfilesInput,
) -> Result<Vec<CharacterProfileSummary>, String> {
    state
        .character_profile_overview_read_application()
        .list(user_id, input.project_id)
        .await
        .map_err(app_error_message)
}

pub async fn delete_profile(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<CharacterProfileDeleteResult, String> {
    let project_id = state
        .character_profile_overview_read_application()
        .profile_project_id(user_id, profile_id)
        .await
        .map_err(app_error_message)?;
    let trace_id = crate::sidecar_clients::character_profile_delete_rag_operation_trace_id(
        project_id, profile_id,
    )?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        state
            .character_profile_deletion_application()
            .delete(user_id, profile_id),
    )
    .await
    .map_err(app_error_message)
}

async fn profile_rag_sync_warnings(state: &AppState, project_id: i64) -> Vec<String> {
    crate::rag_ingest::sync_protagonist_rag_records(state, project_id)
        .await
        .err()
        .map(|error| profile_rag_warnings(vec![error]))
        .unwrap_or_default()
}

fn profile_rag_warnings(errors: Vec<String>) -> Vec<String> {
    if errors.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "角色资料检索索引未完全更新：{}。正式资料未受影响，可稍后在项目操作中重新同步",
            errors.into_iter().take(8).collect::<Vec<_>>().join("；")
        )]
    }
}

pub async fn get_character_arc_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<Value>, String> {
    state
        .character_profile_overview_read_application()
        .character_arc_by_name(user_id, project_id, name)
        .await
        .map_err(app_error_message)
}

pub async fn get_character_voice_fingerprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<Value>, String> {
    state
        .character_profile_overview_read_application()
        .voice_fingerprint_by_name(user_id, project_id, name)
        .await
        .map_err(app_error_message)
}
