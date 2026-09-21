use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatMessage;

use super::super::super::super::contracts::{
    COMPRESSION_FULL_RAW, LlmContextPackRef, LlmContextPackRequirement,
};
use super::super::super::super::planner::estimate_context_tokens;
use a4n_domain::source_text_hash as source_content_sha256;

pub(super) fn system_prompt_requirement(
    system_messages: &[ChatMessage],
) -> Result<Option<LlmContextPackRequirement>, AppError> {
    if system_messages.is_empty() {
        return Ok(None);
    }
    let system_tokens = estimate_context_tokens(
        &serde_json::to_string(system_messages).map_err(AppError::Serialization)?,
    );
    Ok(Some(LlmContextPackRequirement {
        pack_kind: "system_prompt_prefix".to_string(),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextPackRef {
            pack_kind: "system_prompt_prefix".to_string(),
            content_sha256: source_content_sha256(
                &serde_json::to_string(system_messages).map_err(AppError::Serialization)?,
            ),
            compression_level: COMPRESSION_FULL_RAW.to_string(),
            token_estimate: system_tokens,
        }],
    }))
}
