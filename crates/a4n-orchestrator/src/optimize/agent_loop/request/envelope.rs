use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use serde_json::{Value, json};

use super::step::build_agent_loop_step_user_message;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_llm::trace::context_aware_chat_request_trace_json;

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
    let manifest = build_agent_loop_context_manifest(base_user_message, step_user_message)?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.2),
        None,
        Some(llm_token_profiles::JSON_STANDARD.into()),
    )?
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
    }))
}

fn build_agent_loop_context_manifest(
    base_user_message: &str,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let base_user_message = base_user_message.trim();
    if base_user_message.is_empty() {
        return Err(AppError::InvalidArgument(
            "Agent Loop 缺少基础任务输入".to_string(),
        ));
    }
    let mut ledger = LlmContextLedger::new();
    let base_hash = source_content_sha256(base_user_message);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
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
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({"kind": "base_task", "base_task": base_user_message}),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({"kind": "base_task", "base_task": base_user_message}),
            ),
        ],
    })?;
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "Agent Loop 每步必须只返回严格 JSON 决策对象；action 只能是 think、run_tool 或 final。"
                .to_string(),
        ),
    ))
}

pub(in crate::optimize) fn agent_loop_request_payload(
    request: &ChatRequest,
) -> Result<String, AppError> {
    Ok(serde_json::to_string_pretty(
        &context_aware_chat_request_trace_json(request)?,
    )?)
}
