mod budget;
mod model_ref;

use super::contracts::{LlmContextPackDraft, LlmContextPackDraftInput};
pub(super) use super::{canonical_json_text, estimate_context_tokens};
use a4n_core::error::AppError;
use a4n_core::source_spans::source_text_hash as source_content_sha256;

pub use budget::ContextBudgetPlanner;

impl LlmContextPackDraft {
    pub fn from_json(input: LlmContextPackDraftInput) -> Result<Self, AppError> {
        let canonical_content_text = canonical_json_text(&input.canonical_content_json)?;
        let token_estimate = estimate_context_tokens(&canonical_content_text);
        let content_sha256 = source_content_sha256(&canonical_content_text);
        Ok(Self {
            project_id: input.project_id,
            root_project_id: input.root_project_id,
            scope_kind: input.scope_kind,
            workflow: input.workflow,
            pack_kind: input.pack_kind,
            source_refs_json: input.source_refs_json,
            config_hash: input.config_hash,
            canonical_content_json: input.canonical_content_json,
            canonical_content_text,
            content_sha256,
            compression_level: input.compression_level,
            token_estimate,
        })
    }
}

pub use model_ref::context_pack_model_ref;
