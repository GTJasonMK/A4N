mod character_ai;
mod implicit_review;
mod input;
mod inspiration;
mod optimize;
mod records;

pub(super) use character_ai::{
    check_character_conflict, discover_character_profiles,
    preview_check_character_conflict_request, preview_discover_character_profiles_request,
    preview_sync_all_character_profiles_request, preview_sync_character_from_chapters_request,
    sync_all_character_profiles, sync_character_from_chapters,
};
pub(super) use implicit_review::{
    apply_implicit_update_decision, check_implicit_update, get_implicit_review,
    preview_check_implicit_update_request,
};
pub(super) use inspiration::{get_inspiration_history, preview_inspiration_request};
pub(super) use optimize::{
    apply_optimization_patches, create_optimization_analysis_session,
    create_optimization_chat_session, delete_optimize_session, discard_pending_optimize_patch_sets,
    get_optimize_session, list_optimize_patch_sets, list_optimize_sessions, list_optimize_turns,
    prepare_optimization_analysis_session, preview_create_optimization_analysis_session_request,
    preview_prepare_optimization_analysis_session_request,
    preview_propose_optimization_patches_request, preview_send_optimization_message_request,
    record_optimize_agent_event,
};
pub(super) use records::{
    add_profile_attribute, attribute_history, check_profile_sync_conflict, create_profile,
    delete_profile, diff_profile_chapters, execute_attribute_deletion, get_character_arc_status,
    get_character_voice_fingerprint, get_profile_state_at_chapter, get_snapshot, list_behaviors,
    list_deletion_marks, list_profiles, list_snapshots, modify_profile_attribute,
    request_attribute_deletion, reset_deletion_marks, rollback_to_chapter,
    update_character_profile,
};
