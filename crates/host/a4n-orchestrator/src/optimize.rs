//! 内容优化编排：分析、对话 Agent Loop、局部补丁与会话账本。

mod agent_loop;

mod analysis;
mod application;
mod chat;

mod context;
mod contracts;
mod generation;
mod messages;
mod patch;

mod permissions;
pub(crate) mod runtime_tools;
mod service;
mod session_events {
    mod lifecycle;
    mod messages;
    mod patches;
}
mod session_names;
mod turns;
mod workflow_rules;

pub(in crate::optimize) use agent_loop::executor::{
    AgentLoopExecutionError, AgentLoopGenerationContext,
};
pub(crate) use chat::OptimizeChatGenerationApplicationContext;
pub(in crate::optimize) use contracts::{
    OptimizeChatTurnExecution, OptimizeProposePatchesInput, OptimizeSendUserMessageInput,
};
pub(in crate::optimize) use contracts::{PatchPromptBundle, PatchRequestBase};
pub(in crate::optimize) use generation::{
    agent_loop_error_to_generation_error, app_error_to_generation_error,
    generation_stream_completion, optimize_progress,
};
pub(crate) use patch::OptimizationPatch;
pub(crate) use patch::generation::OptimizePatchGenerationApplicationContext;
pub(in crate::optimize) use patch::request::{
    build_content_optimization_patch_request, optimize_llm_call_input_fingerprint_json,
};
pub(crate) use patch::{
    OptimizationPatchProposal, apply_optimization_patches_to_content, optimization_content_hash,
};
pub(crate) use permissions::{optimization_agent_variables, optimization_analysis_variables};
pub(crate) use service::optimize_db_error;
pub(in crate::optimize) use session_names::required_available_session_name;
pub(crate) use workflow_rules::optimize_session_workflow_run_input;

use crate::OrchestratorContext;
pub(crate) use a4n_api_contract::optimize::OptimizationAgentPermissions;

const OPTIMIZATION_CHAT_HISTORY_LIMIT: usize = 12;
const OPTIMIZATION_CHAT_CONTENT_LIMIT: usize = 6000;
const OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS: usize = 2000;
const OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS: usize = 4000;
const OPTIMIZATION_PATCH_MAX_COUNT: usize = 20;

#[cfg(test)]
mod tests;
