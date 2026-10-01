use std::collections::HashMap;

use a4n_db::inspiration::InspirationProjectContextRows;
use a4n_domain::inspiration::InspirationMessageRecord;
use a4n_domain::source_text_hash as source_content_sha256;
use a4n_error::AppError;
use a4n_inspiration_core::inspiration_messages_llm_context;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    context_aware_system_user_request,
};
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest, LlmTraceContext};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::json;

use super::context::inspiration_project_context;
use super::validation::required_message_content;

pub(super) async fn build_inspiration_request_from_context(
    project_id: i64,
    user_input: &str,
    prompt_renderer: &dyn PromptRenderer,
    history: &[InspirationMessageRecord],
    project_context_rows: InspirationProjectContextRows,
) -> Result<ChatRequest, AppError> {
    let user_input = required_message_content(user_input)?;
    let variables = HashMap::new();
    let system_content = prompt_renderer
        .render_prompt("inspiration", &variables, Some(project_id))
        .await?;
    let project_context = inspiration_project_context(project_context_rows);
    let mut context_variables = HashMap::new();
    context_variables.insert(
        "project_context_json".to_string(),
        json!("参见同一稳定上下文包中的 project_context 字段。"),
    );
    let project_context_guard = prompt_renderer
        .render_prompt(
            "inspiration_project_context_guard",
            &context_variables,
            Some(project_id),
        )
        .await?;
    let project_context_hash = source_content_sha256(&serde_json::to_string(&project_context)?);
    let guard_hash = source_content_sha256(&project_context_guard);
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: "inspiration_conversation".into(),
        pack_kind: "inspiration_project_context".into(),
        source_refs_json: Some(json!({
            "project_id": project_id,
            "project_context_sha256": project_context_hash,
            "guard_sha256": guard_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "inspiration_conversation_context_v2",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({
                "guard": project_context_guard,
                "project_context": project_context.clone(),
            }),
        )],
    })?;
    let conversation = build_inspiration_conversation_messages(history, user_input)?;
    let history_hash = source_content_sha256(&serde_json::to_string(
        &inspiration_messages_llm_context(history),
    )?);
    let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
        "project_id": project_id,
        "project_context_sha256": project_context_hash,
        "history_sha256": history_hash,
        "user_input": user_input,
    }))?);
    let manifest = ledger.into_manifest(
        "根据后续对话历史和最新用户消息继续灵感对话；已有项目事实在 inspiration_project_context.project_context。",
        Some(
            "灵感对话必须返回符合 inspiration 系统提示要求的 JSON 对象；回答必须优先尊重稳定上下文包 inspiration_project_context 中的项目事实。"
                .to_string(),
        ),
    );

    let mut request = context_aware_system_user_request(system_content, manifest, Some(0.8), None)?
        .with_stream_usage()
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: None,
            workflow: "inspiration_conversation".to_string(),
            step_key: "conversation_turn".to_string(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(format!("project_{project_id}_conversation_turn")),
            input_fingerprint_sha256: Some(fingerprint),
            context_plan_json: None,
            context_pack_refs_json: None,
        });
    request.messages.extend(conversation);
    let last = request.messages.len() - 1;
    request.add_cache_breakpoint(last, request.messages[last].content.len())?;
    Ok(request)
}

pub(super) fn build_inspiration_conversation_messages(
    history: &[InspirationMessageRecord],
    user_input: &str,
) -> Result<Vec<ChatMessage>, AppError> {
    let user_input = required_message_content(user_input)?;
    let mut messages = inspiration_messages_llm_context(history)
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<Vec<ChatMessage>, _>>()?;
    messages.push(ChatMessage::user(user_input));
    Ok(messages)
}
