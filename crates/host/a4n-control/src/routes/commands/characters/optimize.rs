mod adapters;
mod analysis;
mod chat;
mod patches;
mod records;

pub(in crate::routes::commands) use analysis::{
    create_optimization_analysis_session, prepare_optimization_analysis_session,
    preview_create_optimization_analysis_session_request,
    preview_prepare_optimization_analysis_session_request,
};
pub(in crate::routes::commands) use chat::{
    create_optimization_chat_session, preview_send_optimization_message_request,
};
pub(in crate::routes::commands) use patches::{
    apply_optimization_patches, preview_propose_optimization_patches_request,
};
pub(in crate::routes::commands) use records::{
    delete_optimize_session, discard_optimization_patch_set, discard_pending_optimize_patch_sets,
    get_optimize_session, list_optimize_patch_sets, list_optimize_sessions, list_optimize_turns,
    record_optimize_agent_event,
};
