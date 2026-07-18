use super::super::contracts::LlmContextPackRef;
use a4n_core::models::generation::llm_context_pack;

pub fn context_pack_model_ref(model: &llm_context_pack::Model) -> LlmContextPackRef {
    LlmContextPackRef {
        pack_kind: model.pack_kind.clone(),
        content_sha256: model.content_sha256.clone(),
        compression_level: model.compression_level.clone(),
        token_estimate: model.token_estimate.max(0) as usize,
    }
}
