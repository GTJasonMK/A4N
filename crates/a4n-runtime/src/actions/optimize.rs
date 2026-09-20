mod analysis;
mod chat;
mod patches;
mod preview;
mod sessions;

pub use a4n_api_contract::optimize::{
    AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
    CreateOptimizationChatSessionInput, DiscardOptimizationPatchSetInput,
    ListOptimizeSessionsInput, OptimizationAgentPermissions, OptimizationPatch,
    OptimizationPatchOperation, OptimizationPatchSetSummary, OptimizationSessionSummary,
    OptimizationTurnEventSummary, OptimizationTurnSummary, OptimizeSessionInput,
    PrepareOptimizationAnalysisSessionInput, ProposeOptimizationPatchesInput,
    RecordOptimizeAgentEventInput, SendOptimizationMessageInput,
};
pub use analysis::{
    create_optimization_analysis_session, prepare_optimization_analysis_session,
    preview_create_optimization_analysis_session_request,
    preview_prepare_optimization_analysis_session_request,
    start_optimize_analysis_generation_typed,
};
pub use chat::{
    create_optimization_chat_session, preview_send_optimization_message_request,
    start_optimize_chat_generation_typed,
};
pub use patches::{
    apply_optimization_patches, discard_optimization_patch_set,
    discard_pending_optimize_patch_sets, preview_propose_optimization_patches_request,
    start_optimize_patch_generation_typed,
};
pub use sessions::{
    delete_optimize_session, get_optimize_session, list_optimize_patch_sets,
    list_optimize_sessions, list_optimize_turns, record_optimize_agent_event,
};
