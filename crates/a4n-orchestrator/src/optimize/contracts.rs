use a4n_core::models::{ai::optimization_session, chapter::chapter};
use a4n_llm::provider::ChatRequest;

use super::OptimizeAgentPermissions;
use a4n_prompt::rendering::PromptRenderer;

use super::agent_loop_executor::AgentLoopGenerationContext;

pub(in crate::optimize) struct OptimizeSendUserMessageInput<'a, 'generation> {
    pub session_id: i64,
    pub message: &'a str,
    pub permissions_override: Option<&'a OptimizeAgentPermissions>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub workflow_run_id: i64,
    pub generation: &'generation AgentLoopGenerationContext<'generation>,
}

pub(in crate::optimize) struct OptimizeChatTurnExecution {
    pub(in crate::optimize) turn_id: i64,
    pub(in crate::optimize) user_text: String,
    pub(in crate::optimize) final_reply: String,
}

pub(in crate::optimize) struct OptimizeProposePatchesInput<'a, 'generation> {
    pub session_id: i64,
    pub issue_ids: Option<Vec<String>>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub agent_instruction_override: Option<&'a str>,
    pub workflow_run_id: i64,
    pub generation: &'generation AgentLoopGenerationContext<'generation>,
}

pub(in crate::optimize) struct PatchRequestBase {
    pub(in crate::optimize) session: optimization_session::Model,
    pub(in crate::optimize) chapter: chapter::Model,
    pub(in crate::optimize) override_instruction: Option<String>,
}

pub(in crate::optimize) struct PatchPromptBundle {
    pub(in crate::optimize) request: ChatRequest,
    pub(in crate::optimize) user_message: String,
    pub(in crate::optimize) base_content_hash: String,
    pub(in crate::optimize) issue_count: usize,
    pub(in crate::optimize) permissions: OptimizeAgentPermissions,
}
