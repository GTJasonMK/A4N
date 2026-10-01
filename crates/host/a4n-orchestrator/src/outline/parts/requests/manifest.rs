use a4n_domain::chapter::PartOutlineRecord;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    LlmContextManifest,
};
use serde_json::{Value, json};

use super::super::super::context::{blueprint_context_pack, part_context_json};
use super::super::super::part_contract::PART_GENERATION_OUTPUT_CONTRACT;

pub(super) fn build_part_outline_context_manifest(
    blueprint_context: &Value,
    previous_parts: &[PartOutlineRecord],
    dynamic_user_content: String,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(blueprint_context_pack(blueprint_context)?)?;

    if !previous_parts.is_empty() {
        let previous_part_context = previous_parts
            .iter()
            .map(part_context_json)
            .collect::<Vec<_>>();
        let previous_hash = source_content_sha256(&serde_json::to_string(&previous_part_context)?);
        ledger.add_pack(LlmContextLedgerPackInput {
            scope_kind: "project".into(),
            pack_kind: "outline_previous_part_outlines".into(),
            source_refs_json: Some(json!({
                "source": "part_outline",
                "content_sha256": previous_hash,
                "part_numbers": previous_parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
            })),
            config_identity_json: json!({
                "ledger_version": "outline_part_context_v2",
                "kind": "previous_part_outlines",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({"kind": "previous_part_outlines", "parts": previous_part_context}),
            )],
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
