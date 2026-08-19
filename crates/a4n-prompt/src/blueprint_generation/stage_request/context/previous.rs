use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};

use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::{BLUEPRINT_CONTEXT_LEDGER_VERSION, blueprint_generation_source_hash};

pub(super) fn previous_stage_pack(
    previous_context: &Value,
    stage_key: &str,
) -> Result<LlmContextLedgerPackInput, AppError> {
    let previous_hash = blueprint_generation_source_hash(previous_context)?;
    let previous_content = json!({
        "kind": "previous_stage_outputs",
        "previous_context": previous_context,
    });
    Ok(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "blueprint_previous_stage_outputs".into(),
        source_refs_json: Some(json!({
            "source": "blueprint_generation_previous_stage_outputs",
            "previous_context_sha256": previous_hash,
            "stage_key": stage_key,
        })),
        config_identity_json: json!({
            "ledger_version": BLUEPRINT_CONTEXT_LEDGER_VERSION,
            "kind": "previous_stage_outputs",
            "stage_key": stage_key,
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, previous_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, previous_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, previous_content),
        ],
    })
}
