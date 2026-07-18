use serde_json::Value;

use super::super::{
    ApiError, ServerState, character_records_actions, i64_arg, string_arg, to_value,
};
use super::input::character_web_input_command;

character_web_input_command!(
    create_profile,
    character_records_actions::CreateProfileInput,
    character_records_actions::create_profile,
);

character_web_input_command!(
    list_profiles,
    character_records_actions::ListProfilesInput,
    character_records_actions::list_profiles,
);

pub(in crate::routes::commands) async fn delete_profile(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let profile_id = i64_arg(args, "profileId")?;
    let result = character_records_actions::delete_profile(state.runtime(), user_id, profile_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

character_web_input_command!(
    add_profile_attribute,
    character_records_actions::AddProfileAttributeInput,
    character_records_actions::add_profile_attribute,
);

character_web_input_command!(
    modify_profile_attribute,
    character_records_actions::ModifyProfileAttributeInput,
    character_records_actions::modify_profile_attribute,
);

character_web_input_command!(
    attribute_history,
    character_records_actions::AttributeHistoryInput,
    character_records_actions::attribute_history,
);

pub(in crate::routes::commands) async fn get_character_arc_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let value = character_records_actions::get_character_arc_status(
        state.runtime(),
        user_id,
        project_id,
        name,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(value)
}

pub(in crate::routes::commands) async fn get_character_voice_fingerprint(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let name = string_arg(args, "name")?;
    let value = character_records_actions::get_character_voice_fingerprint(
        state.runtime(),
        user_id,
        project_id,
        name,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(value)
}

character_web_input_command!(
    list_snapshots,
    character_records_actions::ListSnapshotsInput,
    character_records_actions::list_snapshots,
);

character_web_input_command!(
    get_profile_state_at_chapter,
    character_records_actions::GetProfileStateAtChapterInput,
    character_records_actions::get_profile_state_at_chapter,
);

character_web_input_command!(
    get_snapshot,
    character_records_actions::GetSnapshotInput,
    character_records_actions::get_snapshot,
);

character_web_input_command!(
    diff_profile_chapters,
    character_records_actions::DiffProfileChaptersInput,
    character_records_actions::diff_profile_chapters,
);

character_web_input_command!(
    check_profile_sync_conflict,
    character_records_actions::CheckProfileSyncConflictInput,
    character_records_actions::check_profile_sync_conflict,
);

character_web_input_command!(
    rollback_to_chapter,
    character_records_actions::RollbackToChapterInput,
    character_records_actions::rollback_to_chapter,
);

character_web_input_command!(
    list_behaviors,
    character_records_actions::ListBehaviorsInput,
    character_records_actions::list_behaviors,
);

character_web_input_command!(
    list_deletion_marks,
    character_records_actions::ListDeletionMarksInput,
    character_records_actions::list_deletion_marks,
);

character_web_input_command!(
    request_attribute_deletion,
    character_records_actions::RequestAttributeDeletionInput,
    character_records_actions::request_attribute_deletion,
);

character_web_input_command!(
    execute_attribute_deletion,
    character_records_actions::ExecuteAttributeDeletionInput,
    character_records_actions::execute_attribute_deletion,
);

character_web_input_command!(
    reset_deletion_marks,
    character_records_actions::ResetDeletionMarksInput,
    character_records_actions::reset_deletion_marks,
);
