use a4n_error::AppError;
use serde_json::json;

use super::{
    COMPRESSION_FULL_RAW, ChapterVideoSourceContext, LlmContextLedger, LlmContextLedgerPackDraft,
    VIDEO_CONTEXT_LEDGER_VERSION, VIDEO_CONTEXT_SCOPE, source_content_sha256,
};

pub(super) fn add_chapter_source_pack(
    ledger: &mut LlmContextLedger,
    source: &ChapterVideoSourceContext<'_>,
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(source.content);
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: "video_chapter_source".into(),
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
        // 原文用于核实对白和可见事实；不再重复发送分句、摘要与整份分析。
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"content": source.content}),
        )],
    })
}
