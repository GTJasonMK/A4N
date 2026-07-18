use super::access::{find_profile_by_name_for_user, require_profile_for_user};
use super::contracts::{
    CreateProfileInput, DeleteProfileResultSummary, ListProfilesInput, ProtagonistProfileSummary,
};
use super::projections::profile_summary;
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_orchestrator::character::CharacterProfileRecordDeletionResult;
use serde_json::Value;

pub async fn create_profile(
    state: &AppState,
    user_id: i64,
    input: CreateProfileInput,
) -> Result<ProtagonistProfileSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = state
        .orchestrator
        .create_profile(input.project_id, &input.name, input.base_attributes)
        .await
        .map_err(app_error_message)?;
    profile_summary(&profile)
}

pub async fn list_profiles(
    state: &AppState,
    user_id: i64,
    input: ListProfilesInput,
) -> Result<Vec<ProtagonistProfileSummary>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profiles = state
        .orchestrator
        .profiles_by_project(input.project_id)
        .await
        .map_err(app_error_message)?;
    profiles.iter().map(profile_summary).collect()
}

pub async fn delete_profile(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<DeleteProfileResultSummary, String> {
    let profile = require_profile_for_user(state, user_id, profile_id).await?;
    let trace_id = crate::sidecar_clients::character_profile_delete_rag_operation_trace_id(
        profile.project_id,
        profile.id,
    )?;
    let deleted = crate::sidecar_clients::scope_rag_operation_trace(trace_id, async {
        state
            .orchestrator
            .delete_character_profile_records(profile_id)
            .await
            .map_err(|error| error.to_string())
    })
    .await?;
    let CharacterProfileRecordDeletionResult { deletion, rag } = deleted;
    let a4n_orchestrator::rag::CharacterProfileRagDeletionSummary {
        deleted_profile_rag,
        deleted_state_rag,
        deleted_attribute_event_rag,
    } = rag;

    let deleted_portrait_files = deletion.portrait_paths.len();
    for path in deletion.portrait_paths {
        crate::file_utils::remove_data_file_in_data_dir(
            &state.config.data_dir,
            &path,
            "角色立绘图片",
        )?;
    }

    Ok(DeleteProfileResultSummary {
        deleted_profile_rag,
        deleted_state_rag,
        deleted_attribute_event_rag,
        deleted_profile_id: deletion.profile_id,
        deleted_portrait_files,
    })
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
