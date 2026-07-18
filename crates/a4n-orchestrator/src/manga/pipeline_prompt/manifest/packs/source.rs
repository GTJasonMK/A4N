use a4n_core::error::AppError;
use serde_json::json;

use a4n_core::source_spans::SourceSpan;
use a4n_core::source_spans::source_span_prompt_values;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
};

pub(in crate::manga::pipeline_prompt::manifest) fn add_manga_source_pack(
    ledger: &mut LlmContextLedger,
    chapter_content: &str,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(chapter_content);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("manga_chapter_source").into(),
        source_refs_json: Some(json!({
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len()
        })),
        config_identity_json: json!({
            "ledger_version": "manga_context_ledger_v1",
            "kind": "chapter_source",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "chapter_source",
                    "content": chapter_content,
                    "source_spans": source_span_prompt_values(source_spans)
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "chapter_source",
                    "source_spans": source_span_prompt_values(source_spans)
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "chapter_source",
                    "content_sha256": content_sha256,
                    "source_span_count": source_spans.len()
                }),
            ),
        ],
    })
}
