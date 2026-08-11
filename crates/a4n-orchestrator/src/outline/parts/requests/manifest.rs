use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft,
};

use super::super::super::context::{
    focused_part_context, minimal_blueprint_context, minimal_part_context, part_context_json,
};
use super::super::super::part_contract::PART_GENERATION_OUTPUT_CONTRACT;

pub(super) fn build_part_outline_context_manifest(
    blueprint_context: &Value,
    previous_parts: &[part_outline::Model],
    dynamic_user_content: String,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(blueprint_context)?);
    let blueprint_available = blueprint_context["blueprint_available"]
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
            "ledger_version": "outline_part_context_v1",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": minimal_blueprint_context(blueprint_context),
                }),
            ),
        ],
    })?;

    if !previous_parts.is_empty() {
        let previous_part_context = previous_parts
            .iter()
            .map(part_context_json)
            .collect::<Vec<_>>();
        let previous_hash = source_content_sha256(&serde_json::to_string(&previous_part_context)?);
        ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
            scope_kind: ("project").into(),
            pack_kind: ("outline_previous_part_outlines").into(),
            source_refs_json: Some(json!({
                "source": "part_outline",
                "content_sha256": previous_hash,
                "part_numbers": previous_parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
            })),
            config_identity_json: json!({
                "ledger_version": "outline_part_context_v1",
                "kind": "previous_part_outlines",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_part_context,
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FOCUSED_RAW,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_parts
                            .iter()
                            .map(focused_part_context)
                            .collect::<Vec<_>>(),
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_parts
                            .iter()
                            .map(minimal_part_context)
                            .collect::<Vec<_>>(),
                    }),
                ),
            ],
        })?;
    }

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "第 {part_num} 分部（第 {start_ch}-{end_ch} 章）必须返回严格 JSON 对象。{}",
            PART_GENERATION_OUTPUT_CONTRACT
        )),
    ))
}
