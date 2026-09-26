use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    LlmContextManifest,
};
use serde_json::json;

use crate::outline::context::{
    ChapterOutlineGenerationContext, blueprint_context_pack, part_context_json,
};

pub(super) fn build_chapter_outline_context_manifest(
    context: &ChapterOutlineGenerationContext,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(blueprint_context_pack(&context.blueprint_payload)?)?;

    let part_context = context
        .parts
        .iter()
        .map(part_context_json)
        .collect::<Vec<_>>();
    let part_hash = source_content_sha256(&serde_json::to_string(&part_context)?);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "outline_part_outlines".into(),
        source_refs_json: Some(json!({
            "source": "part_outline",
            "content_sha256": part_hash,
            "part_numbers": context.parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v2",
            "kind": "part_outlines",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"kind": "part_outlines", "parts": part_context}),
        )],
    })?;

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some("章节大纲必须返回严格 JSON 对象；根对象必须且只能包含 chapters。".to_string()),
    ))
}
