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
    // ponytail: UTF-8 bytes conservatively bound text tokens; use model tokenizers
    // when recovering unused context capacity is worth their model data/dependency.
    text.len()
}

#[cfg(test)]
mod core_tests {
    use super::{canonical_json_text, estimate_context_tokens};
    use serde_json::json;

    #[test]
    fn preserves_context_token_estimate_and_canonical_json_contracts() {
        assert_eq!(estimate_context_tokens("你好 world"), "你好 world".len());
        assert_eq!(
            canonical_json_text(&json!({"name": "A4N"})).expect("json should serialize"),
            "{\n  \"name\": \"A4N\"\n}"
        );
    }

    #[test]
    fn compact_json_and_unbroken_text_cannot_collapse_to_one_word() {
        let json = serde_json::to_string(&(0..20_000).collect::<Vec<_>>()).unwrap();
        for text in [json, "a".repeat(100_000), "文字".repeat(10_000)] {
            assert_eq!(estimate_context_tokens(&text), text.len());
        }
        assert_eq!(estimate_context_tokens(""), 0);
    }
}

#[cfg(test)]
mod tests;
