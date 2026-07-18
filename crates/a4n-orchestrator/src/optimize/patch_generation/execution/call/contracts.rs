use a4n_core::models::{ai::optimization_session, chapter::chapter};
use a4n_llm::provider::{ChatRequest, ChatResponse};
use serde_json::Value;

use crate::optimize::{OptimizationPatchProposal, streaming::OptimizeLlmStream};

pub(in crate::optimize::patch_generation::execution) struct PatchGenerationCallInput<'a, 'stream> {
    pub(in crate::optimize::patch_generation::execution) session: &'a optimization_session::Model,
    pub(in crate::optimize::patch_generation::execution) chapter: &'a chapter::Model,
    pub(in crate::optimize::patch_generation::execution) turn_id: i64,
    pub(in crate::optimize::patch_generation::execution) base_content_hash: &'a str,
    pub(in crate::optimize::patch_generation::execution) request: ChatRequest,
    pub(in crate::optimize::patch_generation::execution) workflow_run_id: i64,
    pub(in crate::optimize::patch_generation::execution) stream:
        &'a mut Option<OptimizeLlmStream<'stream>>,
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
