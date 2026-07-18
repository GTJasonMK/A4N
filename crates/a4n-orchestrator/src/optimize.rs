//! 内容优化编排：分析、对话 Agent Loop、局部补丁与会话账本。

mod agent_loop;
mod agent_loop_executor;
mod analysis;
mod chat;
mod chat_loop_executor;
mod context;
mod contracts;
mod messages;
mod patch;
mod patch_generation;
mod patch_request;
mod patch_sets;
mod permissions;
mod runtime_tools;
mod service;
mod session_events;
mod session_names;
mod status;
mod streaming;
mod turns;

pub use contracts::{OptimizeProposePatchesInput, OptimizeSendUserMessageInput};
pub(in crate::optimize) use contracts::{PatchPromptBundle, PatchRequestBase};
pub use patch::{
    OptimizationPatch, OptimizationPatchProposal, apply_optimization_patches_to_content,
    optimization_content_hash,
};
pub(in crate::optimize) use patch_request::{
    build_content_optimization_patch_request, optimize_llm_call_input_fingerprint_json,
    response_content_from_checkpoint,
};
pub use permissions::OptimizeAgentPermissions;
pub(in crate::optimize) use service::optimize_db_error;
pub(in crate::optimize) use session_names::required_available_session_name;
pub(in crate::optimize) use status::is_cancelled_optimize_error;
pub use streaming::{OptimizeLlmStream, OptimizeProgress};
pub(in crate::optimize) use streaming::{
    collect_streaming_chat_response, report_optimize_progress,
};

pub use a4n_db::optimize::{
    OptimizationBlueprintRows, OptimizationChapterPatchWriteInput, OptimizationContextReadInput,
    OptimizationContextRows, OptimizationPatchCreateInput, OptimizationPatchSetCreateInput,
    OptimizationSessionCreateInput, OptimizationSessionMessageInsertInput,
    OptimizationSessionUpdateInput, OptimizationTurnCreateInput, OptimizationTurnEventAppendInput,
    OptimizationTurnUpdateInput, OptimizeDbError,
};

use crate::OrchestratorContext;

const OPTIMIZATION_CHAT_HISTORY_LIMIT: usize = 12;
const OPTIMIZATION_CHAT_CONTENT_LIMIT: usize = 6000;
const OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS: usize = 2000;
const OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS: usize = 4000;
const OPTIMIZATION_PATCH_MAX_COUNT: usize = 20;

#[cfg(test)]
mod tests;
