//! 反推请求上下文 manifest。

use a4n_error::AppError;
use serde_json::json;

use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest,
};

use super::{build_reverse_engineer_user_message, reverse_engineer_source_artifact};
use crate::import_reverse::IMPORT_REVERSE_WORKFLOW_KIND;

pub(super) fn build_reverse_engineer_context(
    target: ReverseEngineeringTarget,
    content_sample: &str,
) -> Result<(LlmContextManifest, String), AppError> {
    let prompt_name = target.prompt_name();
    let instruction = target.user_instruction();
    let max_tokens_profile = target.max_tokens_profile();
    let source_artifact = reverse_engineer_source_artifact(content_sample)?;
    let source_hash = source_content_sha256(&serde_json::to_string(&source_artifact)?);
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: IMPORT_REVERSE_WORKFLOW_KIND.into(),
        pack_kind: ("source_artifact").into(),
        source_refs_json: Some(json!({
            "source": "import_content_sample",
            "content_sha256": source_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "import_reverse_engineer_context_v1",
            "prompt_name": prompt_name,
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, source_artifact.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, source_artifact.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, source_artifact),
        ],
    })?;
    let manifest = ledger.into_manifest(
        build_reverse_engineer_user_message(target)?,
        Some(
            "导入反推必须只基于稳定上下文包 source_artifact，不得编造样本之外的项目事实。"
                .to_string(),
        ),
    );
    let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
        "prompt_name": prompt_name,
        "source_artifact_sha256": source_hash,
        "instruction": instruction.trim(),
        "max_tokens_profile": max_tokens_profile,
    }))?);
    Ok((manifest, fingerprint))
}
