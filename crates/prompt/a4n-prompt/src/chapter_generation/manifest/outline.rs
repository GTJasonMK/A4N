//! 章节大纲上下文包。

use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};
use serde_json::json;

use super::compact_required_text;

const FOCUSED_OUTLINE_HEAD_CHAR_LIMIT: usize = 8_000;
const FOCUSED_OUTLINE_TAIL_CHAR_LIMIT: usize = 2_000;
// Context budgets use a conservative UTF-8 byte bound, so the minimal
// variant must remain small enough for CJK text in compact model windows.
const MINIMAL_OUTLINE_HEAD_CHAR_LIMIT: usize = 600;
const MINIMAL_OUTLINE_TAIL_CHAR_LIMIT: usize = 200;

pub(super) fn chapter_outline_pack(outline: &str) -> LlmContextLedgerPackInput {
    let hash = source_content_sha256(outline);
    let focused_outline = compact_required_text(
        outline,
        FOCUSED_OUTLINE_HEAD_CHAR_LIMIT,
        FOCUSED_OUTLINE_TAIL_CHAR_LIMIT,
    );
    let minimal_outline = compact_required_text(
        outline,
        MINIMAL_OUTLINE_HEAD_CHAR_LIMIT,
        MINIMAL_OUTLINE_TAIL_CHAR_LIMIT,
    );
    LlmContextLedgerPackInput {
        scope_kind: "chapter".into(),
        pack_kind: "writing_chapter_outline".into(),
        source_refs_json: Some(json!({"source":"chapter_outline","content_sha256":hash})),
        config_identity_json: json!({"ledger_version":"chapter_writing_context_v2","kind":"chapter_outline"}),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({"kind":"chapter_outline","chapter_outline":outline}),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({"kind":"chapter_outline","chapter_outline":focused_outline}),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({"kind":"chapter_outline","chapter_outline":minimal_outline}),
            ),
        ],
    }
}
