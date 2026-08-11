use a4n_core::error::AppError;
use serde_json::json;

use crate::outline::context::{
    ChapterOutlineGenerationContext, focused_part_context, minimal_blueprint_context,
    minimal_part_context, part_context_json,
};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft,
};

pub(super) fn build_chapter_outline_context_manifest(
    context: &ChapterOutlineGenerationContext,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(&context.blueprint_payload)?);
    let blueprint_available = context.blueprint_payload["blueprint_available"]
        .as_bool()
        .unwrap_or(true);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_blueprint_context").into(),
        source_refs_json: Some(json!({
            "source": if blueprint_available {
                "novel_blueprint_and_formal_materials"
            } else {
                "formal_project_materials"
            },
            "blueprint_available": blueprint_available,
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v1",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": context.blueprint_payload,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": context.blueprint_payload,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": minimal_blueprint_context(&context.blueprint_payload),
                }),
            ),
        ],
    })?;

    let part_context = context
        .parts
        .iter()
        .map(part_context_json)
        .collect::<Vec<_>>();
    let part_hash = source_content_sha256(&serde_json::to_string(&part_context)?);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_part_outlines").into(),
        source_refs_json: Some(json!({
            "source": "part_outline",
            "content_sha256": part_hash,
            "part_numbers": context.parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v1",
            "kind": "part_outlines",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "part_outlines",
                    "parts": part_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "part_outlines",
                    "parts": context.parts.iter().map(focused_part_context).collect::<Vec<_>>(),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "part_outlines",
                    "parts": context.parts.iter().map(minimal_part_context).collect::<Vec<_>>(),
                }),
            ),
        ],
    })?;

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some("章节大纲必须返回严格 JSON 对象；根对象必须且只能包含 chapters。".to_string()),
    ))
}
