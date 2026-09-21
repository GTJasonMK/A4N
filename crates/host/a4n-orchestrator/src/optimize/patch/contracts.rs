use serde::{Deserialize, Serialize};

pub(crate) use a4n_api_contract::optimize::OptimizationPatch;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationPatchProposal {
    pub session_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}
