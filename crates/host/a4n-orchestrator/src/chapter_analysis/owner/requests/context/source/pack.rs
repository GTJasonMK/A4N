use a4n_domain::chapter::ChapterRecord;
use a4n_domain::{
    source_spans::{SourceSpan, source_span_prompt_text},
    source_text_hash,
};
use a4n_error::AppError;
use a4n_llm_context::{COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft};
use serde_json::json;

use crate::chapter_analysis::owner::prompt::chapter_display_title;

pub fn add_chapter_analysis_source_pack(
    ledger: &mut LlmContextLedger,
    ch: &ChapterRecord,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_text_hash(&ch.content);
    let chapter_title = chapter_display_title(ch);
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
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
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({
                "chapter_number": ch.chapter_number,
                "chapter_title": chapter_title,
                "content": source_span_prompt_text(&ch.content, source_spans)
                    .map_err(AppError::InvalidArgument)?,
                "source_reference_format": "⟦id⟧标注其后原文片段的来源 ID；标记不属于原文，标点和段落均保留。",
            }),
        )],
    })
}
