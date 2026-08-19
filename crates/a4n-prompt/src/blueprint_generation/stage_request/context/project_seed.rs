use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};

use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::{BLUEPRINT_CONTEXT_LEDGER_VERSION, blueprint_generation_source_hash};

pub(super) fn project_seed_pack(
    source_input: Value,
) -> Result<LlmContextLedgerPackInput, AppError> {
    let source_hash = blueprint_generation_source_hash(&source_input)?;
    let source_content = json!({
        "kind": "project_seed_context",
        "source_input": source_input,
    });
    Ok(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "blueprint_project_seed_context".into(),
        source_refs_json: Some(json!({
            "source": "blueprint_generation_payload",
            "source_input_sha256": source_hash,
        })),
        config_identity_json: json!({
            "ledger_version": BLUEPRINT_CONTEXT_LEDGER_VERSION,
            "kind": "project_seed_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, source_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, source_content.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, source_content),
        ],
    })
}
