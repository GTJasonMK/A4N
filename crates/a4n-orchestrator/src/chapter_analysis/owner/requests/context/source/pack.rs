use a4n_core::{
    error::AppError,
    models::chapter::chapter,
    source_spans::{SourceSpan, source_span_prompt_values, source_text_hash},
};
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft,
};
use serde_json::json;

use crate::chapter_analysis::owner::prompt::required_chapter_title;

pub fn add_chapter_analysis_source_pack(
    ledger: &mut LlmContextLedger,
    ch: &chapter::Model,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_text_hash(&ch.content);
    let chapter_title = required_chapter_title(ch)?;
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("chapter_analysis_source").into(),
        source_refs_json: Some(json!({
            "chapter_id": ch.id,
            "chapter_number": ch.chapter_number,
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len(),
        })),
        config_identity_json: json!({
            "ledger_version": "chapter_analysis_context_v1",
            "kind": "chapter_source",
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len(),
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "chapter_source",
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "chapter_title": chapter_title,
                    "content": ch.content,
                    "source_spans": source_span_prompt_values(source_spans),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "chapter_source",
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "chapter_title": chapter_title,
                    "source_spans": source_span_prompt_values(source_spans),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "chapter_source",
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "chapter_title": chapter_title,
                    "content_sha256": content_sha256,
                    "source_span_count": source_spans.len(),
                }),
            ),
        ],
    })
}
