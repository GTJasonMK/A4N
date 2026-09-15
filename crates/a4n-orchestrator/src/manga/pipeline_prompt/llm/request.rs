use std::collections::HashMap;

use a4n_db::workflow;
use a4n_error::AppError;
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use a4n_llm_context::{LlmContextManifest, context_aware_system_user_request};
use serde_json::{Value, json};

use crate::workflow::workflow_checkpoint_contract_app_error;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_prompt::rendering::PromptRenderer;

use super::super::contracts::{MANGA_PIPELINE_WORKFLOW, MangaLlmCallInput, MangaPreparedLlmCall};
async fn build_manga_chat_request(
    prompt_renderer: &dyn PromptRenderer,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
    user_content: LlmContextManifest,
    temperature: f32,
) -> Result<ChatRequest, AppError> {
    let sys = prompt_renderer
        .render_prompt(prompt_name, variables, Some(project_id))
        .await?;
    Ok(
        context_aware_system_user_request(sys, user_content, Some(temperature), None)?
            .with_json_response_format()
            .with_trace_context(LlmTraceContext {
                project_id: Some(project_id),
                root_project_id: None,
                workflow: MANGA_PIPELINE_WORKFLOW.to_string(),
                step_key: prompt_name.to_string(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: None,
                input_fingerprint_sha256: None,
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}

pub(in crate::manga) async fn prepare_manga_llm_call(
    input: MangaLlmCallInput<'_>,
) -> Result<MangaPreparedLlmCall, AppError> {
    let MangaLlmCallInput {
        prompt_renderer,
        prompt_name,
        project_id,
        workflow_run_id,
        call_key,
        call_order,
        variables,
        user_content,
        temperature,
    } = input;
    let mut request = build_manga_chat_request(
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        user_content,
        temperature,
    )
    .await?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = workflow_run_id;
        trace_context.call_key = Some(call_key.clone());
    }
    let fingerprint_request_json = context_aware_chat_request_trace_json(&request)?;
    let input_fingerprint_json = json!({
        "prompt_name": prompt_name,
        "project_id": project_id,
        "request": fingerprint_request_json,
    });
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let request_json = context_aware_chat_request_trace_json(&request)?;
    let messages_json = serde_json::to_value(&request.messages).map_err(|err| {
        AppError::InvalidArgument(format!("漫画流水线 LLM 消息序列化失败: {err}"))
    })?;
    Ok(MangaPreparedLlmCall {
        request,
        request_json,
        messages_json,
        call_key,
        call_order,
        input_fingerprint_json,
    })
}
