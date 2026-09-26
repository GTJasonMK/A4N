use a4n_domain::project::BLUEPRINT_REFINEMENT_FIELDS;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    LlmContextManifest,
};
use serde_json::{Value, json};

pub(super) fn build_blueprint_refine_context_manifest(
    blueprint_context: &Value,
    inspiration_context: &Value,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    if !blueprint_context["blueprint"].is_object()
        || !blueprint_context["characters"].is_array()
        || !blueprint_context["relationships"].is_array()
    {
        return Err(AppError::InvalidArgument(
            "蓝图优化上下文必须包含 blueprint 对象、characters 数组和 relationships 数组".into(),
        ));
    }
    let messages = inspiration_context["messages"].as_array().ok_or_else(|| {
        AppError::InvalidArgument("蓝图优化灵感上下文必须包含 messages 数组".into())
    })?;
    for (index, message) in messages.iter().enumerate() {
        if !message["role"].is_string() || !message["content"].is_string() {
            return Err(AppError::InvalidArgument(format!(
                "蓝图优化灵感上下文 messages[{index}] 的 role 和 content 必须是字符串"
            )));
        }
    }

    let mut ledger = LlmContextLedger::new();
    // 灵感历史先于反复精修的蓝图，保留共同前缀；预算不足时明确失败，不截掉用户约束。
    let inspiration_hash = source_content_sha256(&serde_json::to_string(inspiration_context)?);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "inspiration_conversation_context".into(),
        source_refs_json: Some(json!({
            "source": "inspiration_conversation",
            "content_sha256": inspiration_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "blueprint_refine_context_v3",
            "kind": "inspiration_conversation_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({
                "kind": "inspiration_conversation_context",
                "conversation": inspiration_context,
            }),
        )],
    })?;
    let blueprint_hash = source_content_sha256(&serde_json::to_string(blueprint_context)?);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "existing_blueprint_context".into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "blueprint_refine_context_v3",
            "kind": "existing_blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"kind": "existing_blueprint_context", "blueprint_context": blueprint_context}),
        )],
    })?;
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "蓝图优化必须返回严格 JSON 对象；允许字段为 {}；必须同时参考 inspiration_conversation_context 和 existing_blueprint_context。",
            BLUEPRINT_REFINEMENT_FIELDS.join(", ")
        )),
    ))
}
