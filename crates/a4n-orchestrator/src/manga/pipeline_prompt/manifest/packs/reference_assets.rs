use a4n_core::error::AppError;
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};

pub(in crate::manga::pipeline_prompt::manifest) fn add_manga_reference_assets_pack(
    ledger: &mut LlmContextLedger,
    reference_assets: &Value,
) -> Result<(), AppError> {
    let serialized = serde_json::to_string(reference_assets).map_err(AppError::Serialization)?;
    let sha256 = source_content_sha256(&serialized);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("manga_reference_assets").into(),
        source_refs_json: None,
        config_identity_json: json!({
            "ledger_version": "manga_context_ledger_v1",
            "kind": "reference_assets",
            "sha256": sha256
        }),
        required: false,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, reference_assets.clone()),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({ "reference_assets_sha256": sha256 }),
            ),
        ],
    })
}
