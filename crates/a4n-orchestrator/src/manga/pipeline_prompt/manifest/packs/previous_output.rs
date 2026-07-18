use a4n_core::error::AppError;
use serde_json::json;

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
};

use super::super::super::contracts::MangaContextPreviousOutput;

pub(in crate::manga::pipeline_prompt::manifest) fn add_manga_previous_output_pack(
    ledger: &mut LlmContextLedger,
    output: &MangaContextPreviousOutput<'_>,
) -> Result<(), AppError> {
    let raw_sha256 = output.raw_output.map(source_content_sha256);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: (output.pack_kind).into(),
        source_refs_json: Some(json!({
            "prompt_name": output.prompt_name
        })),
        config_identity_json: json!({
            "ledger_version": "manga_context_ledger_v1",
            "kind": "previous_output",
            "pack_kind": output.pack_kind,
            "prompt_name": output.prompt_name,
            "raw_sha256": raw_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "prompt_name": output.prompt_name,
                    "raw_output": output.raw_output,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "prompt_name": output.prompt_name,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "prompt_name": output.prompt_name,
                    "raw_sha256": raw_sha256
                }),
            ),
        ],
    })
}
