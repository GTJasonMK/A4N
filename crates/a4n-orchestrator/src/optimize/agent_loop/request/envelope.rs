use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest, LlmTraceContext};
use serde_json::{Value, json};

use super::step::{agent_loop_rules, build_agent_loop_step_user_message};
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_llm_trace::context_aware_chat_request_trace_json;
use a4n_prompt::request::json_user_message_payload;

const FOCUSED_HISTORY_TURNS: usize = 4;
const MINIMAL_HISTORY_TURNS: usize = 2;
const FOCUSED_HISTORY_CONTENT_LIMIT: usize = 1_200;
const MINIMAL_HISTORY_CONTENT_LIMIT: usize = 600;

pub(in crate::optimize) fn build_first_agent_loop_chat_request(
    project_id: i64,
    session_id: Option<i64>,
    workflow: &str,
    system_prompt: String,
    base_user_message: &str,
) -> Result<ChatRequest, AppError> {
    build_agent_loop_chat_request(
        project_id,
        session_id,
        workflow,
        system_prompt,
        base_user_message,
        &[],
        1,
    )
}

pub(in crate::optimize) fn build_agent_loop_chat_request(
    project_id: i64,
    session_id: Option<i64>,
    workflow: &str,
    system_prompt: String,
    base_user_message: &str,
    observations: &[Value],
    step: usize,
) -> Result<ChatRequest, AppError> {
    let step_user_message = build_agent_loop_step_user_message(observations, step)?;
    let manifest = build_agent_loop_context_manifest(base_user_message)?;
    let mut request = context_aware_system_user_request(system_prompt, manifest, Some(0.2), None)?
        .with_json_response_format()
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: None,
            workflow: workflow.to_string(),
            step_key: session_id
                .map(|id| format!("session_{id}_step_{step}"))
                .unwrap_or_else(|| format!("preview_step_{step}")),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(
                session_id
                    .map(|id| format!("session_{id}_step_{step}"))
                    .unwrap_or_else(|| format!("preview_step_{step}")),
            ),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });
    // Each observation is immutable data in its original role. Subsequent
    // steps append observations rather than rebuilding a growing JSON array.
    for (index, observation) in observations.iter().enumerate() {
        let content = serde_json::to_string(
            &json!({"loop_observation": {"sequence": index + 1, "value": observation}}),
        )?;
        let length = content.len();
        let message_index = request.messages.len();
        request.messages.push(ChatMessage::user(content));
        request.add_cache_breakpoint(message_index, length)?;
    }
    request.messages.push(ChatMessage::user(step_user_message));
    Ok(request)
}

fn build_agent_loop_context_manifest(
    base_user_message: &str,
) -> Result<LlmContextManifest, AppError> {
    let base_user_message = base_user_message.trim();
    if base_user_message.is_empty() {
        return Err(AppError::InvalidArgument(
            "Agent Loop 缺少基础任务输入".to_string(),
        ));
    }
    let mut ledger = LlmContextLedger::new();
    let base_hash = source_content_sha256(base_user_message);
    let (focused_base_task, minimal_base_task) = compressed_base_task_variants(base_user_message);
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: ("optimization_session").into(),
        pack_kind: ("optimization_agent_base_task").into(),
        source_refs_json: Some(json!({
            "source": "optimization_agent_base_task",
            "content_sha256": base_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "optimization_agent_loop_context_v1",
            "kind": "base_task",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({"kind": "base_task", "base_task": base_user_message}),
            ),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, focused_base_task),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal_base_task),
        ],
    })?;
    ledger.add_pack(a4n_llm_context::LlmContextLedgerPackInput {
        scope_kind: "optimization_session".into(),
        pack_kind: "optimization_agent_rules".into(),
        source_refs_json: None,
        config_identity_json: json!({"ledger_version": "optimization_agent_loop_context_v2", "kind": "rules"}),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, agent_loop_rules())],
    })?;
    Ok(ledger.into_manifest(
        String::new(),
        Some(
            "Agent Loop 每步必须只返回严格 JSON 决策对象；action 只能是 think、run_tool 或 final。"
                .to_string(),
        ),
    ))
}

fn compressed_base_task_variants(base_user_message: &str) -> (Value, Value) {
    let Some(payload_text) = json_user_message_payload(base_user_message) else {
        return (
            raw_base_task(base_user_message),
            raw_base_task(base_user_message),
        );
    };
    let Ok(payload) = serde_json::from_str::<Value>(payload_text) else {
        return (
            raw_base_task(base_user_message),
            raw_base_task(base_user_message),
        );
    };

    let focused = json!({
        "kind": "base_task",
        "user_message": payload.get("user_message").cloned().unwrap_or(Value::Null),
        "session": payload.get("session").cloned().unwrap_or(Value::Null),
        "context_manifest": payload.get("context_manifest").cloned().unwrap_or(Value::Null),
        "conversation_history": compact_history(
            payload.get("conversation_history"),
            FOCUSED_HISTORY_TURNS,
            FOCUSED_HISTORY_CONTENT_LIMIT,
        ),
        "write_policy": payload.get("write_policy").cloned().unwrap_or(Value::Null),
    });
    let minimal = json!({
        "kind": "base_task",
        "user_message": payload.get("user_message").cloned().unwrap_or(Value::Null),
        "session": payload.get("session").cloned().unwrap_or(Value::Null),
        "context_manifest": payload.get("context_manifest").cloned().unwrap_or(Value::Null),
        "conversation_history": compact_history(
            payload.get("conversation_history"),
            MINIMAL_HISTORY_TURNS,
            MINIMAL_HISTORY_CONTENT_LIMIT,
        ),
    });
    (focused, minimal)
}

fn raw_base_task(base_user_message: &str) -> Value {
    json!({"kind": "base_task", "base_task": base_user_message})
}

fn compact_history(value: Option<&Value>, max_turns: usize, content_limit: usize) -> Value {
    let Some(turns) = value.and_then(Value::as_array) else {
        return Value::Array(Vec::new());
    };
    let start = turns.len().saturating_sub(max_turns);
    Value::Array(
        turns[start..]
            .iter()
            .map(|turn| {
                let Some(object) = turn.as_object() else {
                    return turn.clone();
                };
                let user = compact_history_text(object.get("user"), content_limit);
                let assistant = compact_history_text(object.get("assistant"), content_limit);
                json!({
                    "turn_id": object.get("turn_id").cloned().unwrap_or(Value::Null),
                    "status": object.get("status").cloned().unwrap_or(Value::Null),
                    "user": user,
                    "assistant": assistant,
                })
            })
            .collect(),
    )
}

fn compact_history_text(value: Option<&Value>, content_limit: usize) -> Value {
    let Some(object) = value.and_then(Value::as_object) else {
        return value.cloned().unwrap_or(Value::Null);
    };
    let Some(content) = object.get("content").and_then(Value::as_str) else {
        return Value::Null;
    };
    let content = truncate_chars(content, content_limit);
    json!({"content": content})
}

fn truncate_chars(content: &str, limit: usize) -> String {
    let mut chars = content.chars();
    let mut truncated = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        truncated.push_str("...[已截断]");
    }
    truncated
}

pub(in crate::optimize) fn agent_loop_request_payload(
    request: &ChatRequest,
) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(
        &context_aware_chat_request_trace_json(request)?,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm_context::preview_context_aware_chat_request;
    use a4n_llm_provider_contract::provider::{MessageRole, PromptCacheMode};

    #[test]
    fn loop_appends_full_observations_after_a_frozen_base_and_budget_counts_them() {
        let observations = vec![
            json!({"action": "run_tool", "success": false, "result": {"evidence": "unique-full-evidence"}}),
            json!({"action": "think", "information_gaps": ["next"]}),
        ];
        let build = |history: &[Value], step| {
            let mut request = build_agent_loop_chat_request(
                7,
                Some(11),
                "optimization",
                "system rules".into(),
                "base task",
                history,
                step,
            )
            .unwrap();
            preview_context_aware_chat_request(&mut request, 1_000_000, 8192).unwrap();
            request
                .configure_prompt_cache("openai-responses", "gpt-5.6-luna", None, Some(7))
                .unwrap();
            request
        };
        let first = build(&observations[..1], 2);
        let next = build(&observations, 3);
        assert_eq!(
            serde_json::to_value(&first.messages[..3]).unwrap(),
            serde_json::to_value(&next.messages[..3]).unwrap()
        );
        assert!(
            next.messages
                .iter()
                .skip(1)
                .all(|message| matches!(message.role, MessageRole::User))
        );
        assert_eq!(next.cache_mode, Some(PromptCacheMode::Explicit));
        assert_eq!(
            next.messages
                .iter()
                .map(|message| message.content.matches("unique-full-evidence").count())
                .sum::<usize>(),
            1
        );
        assert!(
            next.cache_breakpoints
                .iter()
                .any(|point| point.message_index == 3)
        );
        let mut oversized = build_agent_loop_chat_request(
            7,
            Some(11),
            "optimization",
            "system".into(),
            "base",
            &[json!({"evidence": "x".repeat(10_000)})],
            2,
        )
        .unwrap();
        assert!(preview_context_aware_chat_request(&mut oversized, 5_000, 500).is_err());
    }
}
