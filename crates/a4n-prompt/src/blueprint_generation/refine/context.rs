use a4n_domain::project::BLUEPRINT_REFINEMENT_FIELDS;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextLedgerPackInput, LlmContextManifest,
};
use serde_json::{Map, Value, json};

mod minimal;
use minimal::minimal_blueprint_refine_context;

pub(super) fn build_blueprint_refine_context_manifest(
    blueprint_context: &Value,
    inspiration_context: &Value,
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
    let inspiration_hash = source_content_sha256(&serde_json::to_string(inspiration_context)?);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "inspiration_conversation_context".into(),
        source_refs_json: Some(json!({
            "source": "inspiration_conversation",
            "content_sha256": inspiration_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "blueprint_refine_context_v2",
            "kind": "inspiration_conversation_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "inspiration_conversation_context",
                    "conversation": inspiration_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "inspiration_conversation_context",
                    "conversation": compact_inspiration_context(inspiration_context, 24, 4000)?,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "inspiration_conversation_context",
                    "conversation": compact_inspiration_context(inspiration_context, 8, 1200)?,
                }),
            ),
        ],
    })?;
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "蓝图优化必须返回严格 JSON 对象；允许字段为 {}；必须同时参考 inspiration_conversation_context 和 existing_blueprint_context。",
            BLUEPRINT_REFINEMENT_FIELDS.join(", ")
        )),
    ))
}

fn compact_inspiration_context(
    context: &Value,
    max_messages: usize,
    max_content_chars: usize,
) -> Result<Value, AppError> {
    let object = context.as_object().ok_or_else(|| AppError::InvalidState {
        current: "inspiration_context_not_object".to_string(),
        operation: "压缩蓝图优化灵感上下文".to_string(),
    })?;
    let messages = object
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidState {
            current: "inspiration_context.messages_not_array".to_string(),
            operation: "压缩蓝图优化灵感上下文".to_string(),
        })?;
    let mut compact = Vec::new();
    let omitted_message_count = messages.len().saturating_sub(max_messages);
    if messages.len() <= max_messages {
        compact.extend(
            messages
                .iter()
                .map(|message| compact_message(message, max_content_chars)),
        );
    } else {
        let head = max_messages / 2;
        let tail = max_messages - head;
        compact.extend(
            messages
                .iter()
                .take(head)
                .map(|message| compact_message(message, max_content_chars)),
        );
        compact.extend(
            messages
                .iter()
                .skip(messages.len() - tail)
                .map(|message| compact_message(message, max_content_chars)),
        );
    }
    let mut result = Map::new();
    result.insert("messages".to_string(), Value::Array(compact));
    if omitted_message_count > 0 {
        result.insert(
            "omitted_message_count".to_string(),
            json!(omitted_message_count),
        );
    }
    Ok(Value::Object(result))
}

fn compact_message(message: &Value, max_content_chars: usize) -> Value {
    let role = message.get("role").cloned().unwrap_or(Value::Null);
    let content = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let content_length = content.chars().count();
    let compact_content = if content_length <= max_content_chars {
        content.to_string()
    } else {
        let head = max_content_chars / 2;
        let tail = max_content_chars - head;
        let prefix = content.chars().take(head).collect::<String>();
        let suffix = content
            .chars()
            .skip(content_length - tail)
            .collect::<String>();
        format!("{prefix}\n…中间内容已压缩…\n{suffix}")
    };
    json!({"role": role, "content": compact_content})
}
