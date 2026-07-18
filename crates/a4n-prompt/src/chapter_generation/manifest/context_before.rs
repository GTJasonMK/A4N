//! 前文场景状态上下文包。

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};
use serde_json::json;

pub(super) fn context_before_pack(context: &str) -> LlmContextLedgerPackInput {
    let hash = source_content_sha256(context);
    LlmContextLedgerPackInput {
        scope_kind: "chapter".into(),
        pack_kind: "writing_context_before".into(),
        source_refs_json: Some(
            json!({"source":"chapter_generation_context_before","content_sha256":hash}),
        ),
        config_identity_json: json!({"ledger_version":"chapter_writing_context_v1","kind":"context_before"}),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({"kind":"context_before","context_before":context}),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({"kind":"context_before","context_before":context}),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({"kind":"context_before","content_sha256":hash,"char_count":context.chars().count()}),
            ),
        ],
    }
}
