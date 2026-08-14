use std::collections::HashMap;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::user::message;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_db::inspiration::InspirationProjectContextRows;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextLedgerPackInput, context_aware_system_user_request,
};
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use super::context::inspiration_project_context;
use super::validation::required_message_content;

pub async fn build_inspiration_request_from_context(
    project_id: i64,
    user_input: &str,
    prompt_renderer: &dyn PromptRenderer,
    history: &[message::Model],
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
        json!(serde_json::to_string_pretty(&project_context)?),
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
            "ledger_version": "inspiration_conversation_context_v1",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "guard": project_context_guard,
                    "project_context": project_context.clone(),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "guard": project_context_guard,
                    "project_context": project_context.clone(),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "guard": "回答必须优先尊重已有项目事实；不要把数据库技术字段当作故事设定。",
                    "project_context": project_context.clone(),
                }),
            ),
        ],
    })?;
    let dynamic_user_content = build_inspiration_dynamic_user_content(history, user_input)?;
    let history_hash = source_content_sha256(&serde_json::to_string(
        &inspiration_history_payload(history),
    )?);
    let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
        "project_id": project_id,
        "project_context_sha256": project_context_hash,
        "history_sha256": history_hash,
        "user_input": user_input,
    }))?);
    let manifest = ledger.into_manifest(
        dynamic_user_content,
        Some(
            "灵感对话必须返回符合 inspiration 系统提示要求的 JSON 对象；回答必须优先尊重稳定上下文包 inspiration_project_context 中的项目事实。"
                .to_string(),
        ),
    );

    Ok(context_aware_system_user_request(
        system_content,
        manifest,
        Some(0.8),
        None,
        Some(llm_token_profiles::STANDARD.into()),
    )?
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
    }))
}

fn inspiration_history_payload(history: &[message::Model]) -> Vec<Value> {
    history
        .iter()
        .filter_map(|item| match item.role.as_str() {
            "user" | "assistant" => Some(json!({
                "role": item.role,
                "content": item.content,
            })),
            _ => None,
        })
        .collect()
}

pub(super) fn build_inspiration_dynamic_user_content(
    history: &[message::Model],
    user_input: &str,
) -> Result<String, AppError> {
    let user_input = required_message_content(user_input)?;
    let payload = json!({
        "context_refs": {
            "project_context": "稳定上下文包 inspiration_project_context",
        },
        "conversation": {
            "recent_history": inspiration_history_payload(history),
            "latest_user_input": user_input,
        },
        "response_requirement": {
            "format": "strict_json_object",
            "follow_system_prompt": "必须遵守 inspiration 系统提示词中的 JSON 结构、inspired_options 控件规则、conversation_state 和 is_complete 约束。",
            "context_rule": "回答必须结合 latest_user_input 与 recent_history，并优先尊重 inspiration_project_context 中已有项目事实。"
        }
    });
    Ok(format!(
        "继续灵感对话。只返回系统提示词要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。\n\n输入：\n{}",
        serde_json::to_string_pretty(&payload)?
    ))
}
