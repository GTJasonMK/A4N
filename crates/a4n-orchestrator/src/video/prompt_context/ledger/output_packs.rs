use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, VIDEO_CONTEXT_LEDGER_VERSION, VIDEO_CONTEXT_SCOPE,
    VideoContextPreviousOutput, source_content_sha256,
};

pub(super) fn add_previous_output_pack(
    ledger: &mut LlmContextLedger,
    output: &VideoContextPreviousOutput<'_>,
) -> Result<(), AppError> {
    let raw_sha256 = output.raw_output.map(source_content_sha256);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: (output.pack_kind).into(),
        source_refs_json: Some(json!({
            "source_name": output.source_name,
            "placeholder": output.placeholder
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "source_name": output.source_name,
            "pack_kind": output.pack_kind,
            "raw_sha256": raw_sha256,
            "placeholder": output.placeholder
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "source_name": output.source_name,
                    "placeholder": output.placeholder,
                    "raw_output": output.raw_output,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "source_name": output.source_name,
                    "placeholder": output.placeholder,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "source_name": output.source_name,
                    "placeholder": output.placeholder,
                    "raw_sha256": raw_sha256
                }),
            ),
        ],
    })
}

pub(super) fn add_optional_value_pack(
    ledger: &mut LlmContextLedger,
    pack_kind: &str,
    config_identity: Value,
    value: &Value,
) -> Result<(), AppError> {
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: (pack_kind).into(),
        source_refs_json: None,
        config_identity_json: config_identity,
        required: false,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, value.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, json!({
                "value_sha256": source_content_sha256(&serde_json::to_string(value).map_err(AppError::Serialization)?)
            })),
        ],
    })
}

pub(super) fn trimmed_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
