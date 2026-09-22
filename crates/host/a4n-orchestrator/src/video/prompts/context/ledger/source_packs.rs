use a4n_error::AppError;
use serde_json::json;

use super::{
    COMPRESSION_COMPRESSED_SCENE, COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW,
    COMPRESSION_MINIMAL, ChapterVideoSourceContext, LlmContextLedger, LlmContextLedgerPackDraft,
    SourceSpan, VIDEO_CONTEXT_LEDGER_VERSION, VIDEO_CONTEXT_SCOPE,
    final_video_prompt_public_context, source_content_sha256, source_span_prompt_values,
    trimmed_optional,
};

pub(super) fn add_chapter_source_pack(
    ledger: &mut LlmContextLedger,
    source: &ChapterVideoSourceContext<'_>,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(source.content);
    let full = json!({
        "kind": "chapter_source",
        "content": source.content,
        "summary": trimmed_optional(source.summary),
        "analysis": source.analysis,
        "source_spans": source_span_prompt_values(source_spans)
    });
    let focused = json!({
        "kind": "chapter_source",
        "summary": trimmed_optional(source.summary),
        "analysis": source.analysis,
        "source_spans": source_span_prompt_values(source_spans)
    });
    let compressed = json!({
        "kind": "chapter_source",
        "summary": trimmed_optional(source.summary),
        "analysis": source.analysis,
        "source_span_count": source_spans.len(),
        "content_sha256": content_sha256
    });
    let minimal = json!({
        "kind": "chapter_source",
        "summary": trimmed_optional(source.summary),
        "source_span_count": source_spans.len(),
        "content_sha256": content_sha256
    });
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_chapter_source").into(),
        source_refs_json: Some(json!({
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len()
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "chapter_source",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, full),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, focused),
            LlmContextLedgerPackDraft::new(COMPRESSION_COMPRESSED_SCENE, compressed),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal),
        ],
    })
}

pub(super) fn add_public_chapter_source_pack(
    ledger: &mut LlmContextLedger,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(source.content);
    let public_analysis = source.analysis.map(final_video_prompt_public_context);
    let full = json!({
        "kind": "chapter_source_public",
        "content": source.content,
        "summary": trimmed_optional(source.summary),
        "analysis": public_analysis
    });
    let focused = json!({
        "kind": "chapter_source_public",
        "summary": trimmed_optional(source.summary),
        "analysis": public_analysis,
        "content_sha256": content_sha256
    });
    let minimal = json!({
        "kind": "chapter_source_public",
        "summary": trimmed_optional(source.summary),
        "content_sha256": content_sha256
    });
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_chapter_source_public").into(),
        source_refs_json: Some(json!({
            "content_sha256": content_sha256
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "chapter_source_public",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, full),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, focused),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal),
        ],
    })
}
