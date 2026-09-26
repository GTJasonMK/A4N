//! 章节大纲上下文包。

use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::{COMPRESSION_FULL_RAW, LlmContextLedgerPackDraft, LlmContextLedgerPackInput};
use serde_json::json;

pub(super) fn chapter_outline_pack(outline: &str) -> LlmContextLedgerPackInput {
    let hash = source_content_sha256(outline);
    LlmContextLedgerPackInput {
        scope_kind: "chapter".into(),
        pack_kind: "writing_chapter_outline".into(),
        source_refs_json: Some(json!({"source":"chapter_outline","content_sha256":hash})),
        config_identity_json: json!({"ledger_version":"chapter_writing_context_v2","kind":"chapter_outline"}),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"kind":"chapter_outline","chapter_outline":outline}),
        )],
    }
}
