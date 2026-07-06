use std::collections::HashMap;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_engine::llm::provider::{
    ChatRequest, LlmProvider, LlmTraceContext, is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::{Value, json};

use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::workflow::WorkflowRunService;

use super::PortraitService;
use super::design_brief::character_sheet_generation_fields;
use super::types::portrait_type_description;

#[derive(Clone, Copy)]
pub struct PortraitPromptSpec<'a> {
    pub profile_name: &'a str,
    pub appearance: Option<&'a str>,
    pub gender: Option<&'a str>,
    pub age: Option<&'a str>,
    pub personality: Option<&'a str>,
    pub background: Option<&'a str>,
    pub profile_context: Option<&'a Value>,
    pub project_context: Option<&'a Value>,
    pub portrait_type: &'a str,
    pub art_style: Option<&'a str>,
    pub expression: Option<&'a str>,
    pub existing_portrait_count: Option<u64>,
}

pub struct PortraitPromptRequestInput<'a> {
    pub project_id: i64,
    pub spec: PortraitPromptSpec<'a>,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
}

pub struct PortraitPromptGenerationInput<'a> {
    pub project_id: i64,
    pub spec: PortraitPromptSpec<'a>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
}

struct PortraitPromptPayloadInput<'a> {
    spec: PortraitPromptSpec<'a>,
    portrait_type_text: &'a str,
}

impl PortraitService {
    pub fn build_generate_prompt_request(
        &self,
        input: PortraitPromptRequestInput<'_>,
    ) -> Result<ChatRequest, AppError> {
        let PortraitPromptRequestInput {
            project_id,
            spec,
            prompt_engine,
            variables,
        } = input;
        let prompt_content =
            prompt_engine.render("character_portrait_generation", variables, Some(project_id))?;

        let type_desc = portrait_type_description(spec.portrait_type)?;
        let dynamic_payload = portrait_prompt_payload_value(PortraitPromptPayloadInput {
            spec,
            portrait_type_text: type_desc,
        })?;
        let mut user_variables = HashMap::new();
        user_variables.insert(
            "payload_json".to_string(),
            Value::String(serde_json::to_string_pretty(&dynamic_payload)?),
        );
        let user_prompt = prompt_engine.render(
            "character_portrait_generation_user",
            &user_variables,
            Some(project_id),
        )?;
        let stable_context = json!({
            "profile_name": spec.profile_name,
            "appearance": optional_prompt_text(spec.appearance),
            "gender": optional_prompt_text(spec.gender),
            "age": optional_prompt_text(spec.age),
            "personality": optional_prompt_text(spec.personality),
            "background": optional_prompt_text(spec.background),
            "profile_context": spec.profile_context.cloned().unwrap_or_else(|| json!({})),
            "project_context": spec.project_context.cloned().unwrap_or_else(|| json!({})),
        });
        let context_hash = source_content_sha256(&serde_json::to_string(&stable_context)?);
        let mut ledger = LlmContextLedger::new();
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("portrait_prompt_generation").into(),
            pack_kind: ("portrait_profile_project_context").into(),
            source_refs_json: Some(json!({
                "profile_name": spec.profile_name,
                "content_sha256": context_hash,
            })),
            config_identity_json: json!({
                "ledger_version": "portrait_prompt_generation_context_v1",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, stable_context.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, stable_context.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, stable_context),
            ],
        })?;
        let manifest = ledger.into_manifest(
            user_prompt,
            Some("角色画像提示词必须基于稳定上下文包 portrait_profile_project_context，输出只能是可直接用于绘图模型的中文提示词正文。".to_string()),
        );
        let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
            "profile_name": spec.profile_name,
            "context_sha256": context_hash,
            "portrait_type": spec.portrait_type,
            "art_style": optional_prompt_text(spec.art_style),
            "expression": optional_prompt_text(spec.expression),
            "existing_portrait_count": spec.existing_portrait_count,
        }))?);

        Ok(context_aware_system_user_request(
            prompt_content,
            manifest,
            Some(0.8),
            None,
            Some(llm_token_profiles::MEDIA_PROMPT.into()),
        )?
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: None,
            workflow: "portrait_prompt_generation".to_string(),
            step_key: "character_portrait_generation".to_string(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(format!(
                "portrait_prompt:{}:{}",
                spec.profile_name, spec.portrait_type
            )),
            input_fingerprint_sha256: Some(fingerprint),
            context_plan_json: None,
            context_pack_refs_json: None,
        }))
    }

    pub async fn generate_prompt_with_checkpoint(
        &self,
        input: PortraitPromptGenerationInput<'_>,
    ) -> Result<String, AppError> {
        let PortraitPromptGenerationInput {
            project_id,
            spec,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        } = input;
        let mut request = self.build_generate_prompt_request(PortraitPromptRequestInput {
            project_id,
            spec,
            prompt_engine,
            variables,
        })?;

        let call_key = "portrait_prompt.generate".to_string();
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json = portrait_prompt_call_fingerprint_json(
            project_id,
            spec.profile_name,
            spec.portrait_type,
            &request,
        )?;
        let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }

        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, &call_key)
            .await?;
        if let Some(checkpoint) = load_completed_llm_call_checkpoint(
            self.db.clone(),
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )
        .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
            return restored_portrait_prompt_checkpoint(checkpoint_json, &call_key);
        }

        let response = llm.chat(request).await?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "人物三视图提示词输出达到当前 max_tokens 上限，提示词可能被截断；请在设置页调大“媒体提示词”输出档位，或减少角色上下文后重试".to_string(),
            ));
        }

        let output = normalize_portrait_prompt_response(&response.message.content)?;
        let usage = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": output.clone(),
                    "parsed_output": output.clone(),
                    "response_content": response.message.content,
                    "usage": usage,
                    "finish_reason": response.finish_reason,
                }),
                artifacts_json: None,
            },
        )
        .await?;
        Ok(output)
    }
}

pub fn normalize_portrait_prompt_response(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "人物三视图提示词 LLM 响应为空".to_string(),
        ));
    }
    if looks_like_structured_or_fenced_response(trimmed) {
        return Err(AppError::InvalidArgument(format!(
            "人物三视图提示词必须是纯 prompt 文本，不能返回 JSON、Markdown 代码块或思考标签；输出预览：{}",
            portrait_response_preview(trimmed)
        )));
    }
    Ok(trimmed.to_string())
}

fn looks_like_structured_or_fenced_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('#')
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}

fn portrait_response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    trimmed.chars().take(180).collect()
}

fn portrait_prompt_call_fingerprint_json(
    project_id: i64,
    profile_name: &str,
    portrait_type: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "portrait_prompt_call_v1",
        "workflow": "portrait_prompt_generation",
        "call_key": "portrait_prompt.generate",
        "project_id": project_id,
        "profile_name": profile_name,
        "portrait_type": portrait_type,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn restored_portrait_prompt_checkpoint(
    checkpoint_json: Value,
    call_key: &str,
) -> Result<String, AppError> {
    let parsed_output = required_checkpoint_string(&checkpoint_json, call_key, "parsed_output")?;
    let response_content =
        required_checkpoint_string(&checkpoint_json, call_key, "response_content")?;
    let reparsed = normalize_portrait_prompt_response(&response_content)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复人物三视图提示词 LLM 调用断点 {call_key}"),
        });
    }
    Ok(reparsed)
}

fn required_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("missing_or_invalid_checkpoint_field:{field}"),
            operation: format!("读取人物三视图提示词 LLM 调用断点 {call_key}"),
        })
}

#[cfg(test)]
pub(super) fn portrait_prompt_request(
    project_id: i64,
    spec: PortraitPromptSpec<'_>,
    prompt_engine: &dyn PromptEngine,
) -> Result<String, AppError> {
    let payload = portrait_prompt_payload_value(PortraitPromptPayloadInput {
        spec,
        portrait_type_text: spec.portrait_type,
    })?;
    let mut variables = HashMap::new();
    variables.insert(
        "payload_json".to_string(),
        Value::String(serde_json::to_string_pretty(&payload)?),
    );
    prompt_engine.render(
        "character_portrait_generation_user",
        &variables,
        Some(project_id),
    )
}

fn portrait_prompt_payload_value(input: PortraitPromptPayloadInput<'_>) -> Result<Value, AppError> {
    let PortraitPromptPayloadInput {
        spec,
        portrait_type_text,
    } = input;
    let mut payload = character_sheet_generation_fields();
    payload.insert(
        "context_refs".to_string(),
        json!({
            "profile_and_project": "稳定上下文包 portrait_profile_project_context",
        }),
    );
    payload.insert(
        "character_name".to_string(),
        Value::String(spec.profile_name.to_string()),
    );
    payload.insert(
        "portrait_type".to_string(),
        Value::String(portrait_type_text.to_string()),
    );
    payload.insert(
        "appearance".to_string(),
        spec.appearance
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "gender".to_string(),
        spec.gender
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "age".to_string(),
        spec.age
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "personality".to_string(),
        spec.personality
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "background".to_string(),
        spec.background
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_string()))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "profile_context".to_string(),
        spec.profile_context
            .map(|_| json!("稳定上下文包 portrait_profile_project_context.profile_context"))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "project_context".to_string(),
        spec.project_context
            .map(|_| json!("稳定上下文包 portrait_profile_project_context.project_context"))
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "art_style".to_string(),
        optional_prompt_text(spec.art_style)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "expression".to_string(),
        optional_prompt_text(spec.expression)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "existing_portrait_count".to_string(),
        spec.existing_portrait_count
            .map(|count| Value::Number(count.into()))
            .unwrap_or(Value::Null),
    );
    Ok(Value::Object(payload))
}

fn optional_prompt_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}
