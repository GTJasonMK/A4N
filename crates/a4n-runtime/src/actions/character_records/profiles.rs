use super::access::{find_profile_by_name_for_user, require_profile_for_user};
use super::projections::profile_summary;
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_api_contract::character::{
    CharacterProfileCreateResult, CharacterProfileDeleteResult, CharacterProfileMutationResult,
    CharacterProfileSummary, CreateProfileInput, ListProfilesInput, UpdateCharacterProfileInput,
};
use a4n_orchestrator::character::CharacterRecordLifecycleError;
use serde_json::Value;

pub async fn create_profile(
    state: &AppState,
    user_id: i64,
    input: CreateProfileInput,
) -> Result<CharacterProfileCreateResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = state
        .orchestrator()
        .create_profile_validated(input.project_id, input.name, input.base_attributes)
        .await
        .map_err(app_error_message)?;
    Ok(CharacterProfileCreateResult {
        profile_id: profile.id,
        warnings: profile_rag_sync_warnings(state, input.project_id).await,
    })
}

pub async fn update_character_profile(
    state: &AppState,
    user_id: i64,
    input: UpdateCharacterProfileInput,
) -> Result<CharacterProfileMutationResult, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let project_id = input.project_id;
    let profile = state
        .orchestrator()
        .update_initial_profile_validated(
            project_id,
            input.profile_id,
            input.expected_current_state,
            input.base_attributes,
        )
        .await
        .map_err(app_error_message)?;
    Ok(CharacterProfileMutationResult {
        profile: profile_summary(&profile)?,
        warnings: profile_rag_sync_warnings(state, project_id).await,
    })
}

pub async fn list_profiles(
    state: &AppState,
    user_id: i64,
    input: ListProfilesInput,
) -> Result<Vec<CharacterProfileSummary>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profiles = state
        .orchestrator()
        .profiles_by_project(input.project_id)
        .await
        .map_err(app_error_message)?;
    profiles.iter().map(profile_summary).collect()
}

pub async fn delete_profile(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<CharacterProfileDeleteResult, String> {
    let profile = require_profile_for_user(state, user_id, profile_id).await?;
    let trace_id = crate::sidecar_clients::character_profile_delete_rag_operation_trace_id(
        profile.project_id,
        profile.id,
    )?;
    let deleted = crate::sidecar_clients::scope_rag_operation_trace(trace_id, async {
        state
            .orchestrator()
            .delete_character_profile_records_validated(profile_id)
            .await
    })
    .await;
    match deleted {
        Ok(_) => Ok(CharacterProfileDeleteResult { warnings: vec![] }),
        Err(CharacterRecordLifecycleError::Rag(error)) => Ok(CharacterProfileDeleteResult {
            warnings: profile_rag_warnings(vec![error.to_string()]),
        }),
        Err(error) => Err(error.to_string()),
    }
}

async fn profile_rag_sync_warnings(state: &AppState, project_id: i64) -> Vec<String> {
    match crate::rag_ingest::ingest_protagonists_inner(state, project_id, None).await {
        Ok(stats) => profile_rag_warnings(stats.errors),
        Err(error) => profile_rag_warnings(vec![error]),
    }
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
    let profile = find_profile_by_name_for_user(state, user_id, project_id, &name).await?;
    Ok(profile.character_arc)
}

pub async fn get_character_voice_fingerprint(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: String,
) -> Result<Option<Value>, String> {
    let profile = find_profile_by_name_for_user(state, user_id, project_id, &name).await?;
    Ok(profile.voice_fingerprint)
}
