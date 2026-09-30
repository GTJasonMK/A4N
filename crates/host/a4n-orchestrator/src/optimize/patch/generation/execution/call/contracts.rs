use a4n_db_models::models::ai::optimization_session;
use a4n_domain::chapter::ChapterRecord;
use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use crate::optimize::{
    OptimizationPatchProposal, agent_loop::executor::AgentLoopGenerationContext,
};

pub(in crate::optimize::patch::generation::execution) struct PatchGenerationCallInput<
    'a,
    'generation,
> {
    pub(in crate::optimize::patch::generation::execution) session: &'a optimization_session::Model,
    pub(in crate::optimize::patch::generation::execution) chapter: &'a ChapterRecord,
    pub(in crate::optimize::patch::generation::execution) turn_id: i64,
    pub(in crate::optimize::patch::generation::execution) base_content_hash: &'a str,
    pub(in crate::optimize::patch::generation::execution) request: ChatRequest,
    pub(in crate::optimize::patch::generation::execution) workflow_run_id: i64,
    pub(in crate::optimize::patch::generation::execution) generation:
        &'generation AgentLoopGenerationContext<'generation>,
}

pub(super) struct PreparedPatchGenerationCall {
    pub(super) request: ChatRequest,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
}

pub(super) struct RestoredPatchGenerationCall {
    pub(super) response: ChatResponse,
    pub(super) proposal: OptimizationPatchProposal,
}
