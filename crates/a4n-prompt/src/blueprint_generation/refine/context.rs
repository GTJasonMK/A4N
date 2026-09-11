use a4n_domain::project::BLUEPRINT_REFINEMENT_FIELDS;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_llm::context::{
    COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput, LlmContextManifest,
};
use serde_json::{Value, json};

mod minimal;
use minimal::minimal_blueprint_refine_context;

pub(super) fn build_blueprint_refine_context_manifest(
    blueprint_context: &Value,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(blueprint_context)?);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "existing_blueprint_context".into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "blueprint_refine_context_v1",
            "kind": "existing_blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({"kind": "existing_blueprint_context", "blueprint_context": blueprint_context}),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "existing_blueprint_context",
                    "blueprint_context": minimal_blueprint_refine_context(blueprint_context)?,
                }),
            ),
        ],
    })?;
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "蓝图优化必须返回严格 JSON 对象；允许字段为 {}。",
            BLUEPRINT_REFINEMENT_FIELDS.join(", ")
        )),
    ))
}
