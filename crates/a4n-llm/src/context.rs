use a4n_error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod contracts;
mod ledger;
mod manifest;
mod planner;
mod request;

pub use self::contracts::{
    COMPRESSION_COMPRESSED_SCENE, COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW,
    COMPRESSION_MINIMAL, LlmContextLedgerPackDraft, LlmContextLedgerPackInput, LlmContextPackDraft,
    LlmContextPackDraftInput, LlmContextPackRef, LlmContextPackRequirement, LlmContextPlan,
    LlmContextRequestPreview, PlannedContextPack,
};
pub use self::ledger::LlmContextLedger;
pub use self::manifest::{render_context_manifest_user_content, validate_context_manifest};
pub use self::planner::ContextBudgetPlanner;
pub use self::request::{
    ContextAwareJsonLlmRequestInput, chat_request_with_workflow_run_id,
    context_aware_system_user_json_llm_request, context_aware_system_user_llm_request,
    context_aware_system_user_request, preview_context_aware_chat_request,
    replace_request_user_content_from_manifest,
};

pub const CONTEXT_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT: &str = "profile_sync_source_context";
pub const PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT: &str = "implicit_classification_context";
pub const PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT: &str = "implicit_update_review_context";
pub const PACK_KIND_SCENE_PROFILE_SYNC_SOURCE_CONTEXT: &str = "scene_profile_sync_source_context";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextManifest {
    pub schema_version: u32,
    pub packs: Vec<LlmContextManifestPack>,
    pub dynamic_user_content: String,
    pub output_contract: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextManifestPack {
    pub scope_kind: String,
    pub pack_kind: String,
    pub source_refs_json: Option<Value>,
    pub config_hash: String,
    pub required: bool,
    pub stable_prefix: bool,
    pub variants: Vec<LlmContextManifestPackVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContextManifestPackVariant {
    pub compression_level: String,
    pub canonical_content_json: Value,
}

pub fn canonical_json_text(value: &Value) -> Result<String, AppError> {
    serde_json::to_string_pretty(value).map_err(AppError::Serialization)
}

pub fn estimate_context_tokens(text: &str) -> usize {
    let chinese_chars = text.chars().filter(|c| u32::from(*c) > 0x2E80).count();
    let other_words = text.split_whitespace().count();
    ((chinese_chars as f32 * 0.5) + (other_words as f32 * 1.3)).ceil() as usize
}

#[cfg(test)]
mod core_tests {
    use super::{canonical_json_text, estimate_context_tokens};
    use serde_json::json;

    #[test]
    fn preserves_context_token_estimate_and_canonical_json_contracts() {
        assert_eq!(estimate_context_tokens("你好 world"), 4);
        assert_eq!(
            canonical_json_text(&json!({"name": "A4N"})).expect("json should serialize"),
            "{\n  \"name\": \"A4N\"\n}"
        );
    }
}

#[cfg(test)]
mod tests;
