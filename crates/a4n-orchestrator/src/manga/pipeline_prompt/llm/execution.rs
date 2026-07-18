use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_db::workflow;
use a4n_llm::provider::is_truncated_finish_reason;
use serde_json::Value;

use crate::{OrchestratorContext, workflow::workflow_checkpoint_contract_app_error};
use a4n_llm::context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;

use super::super::contracts::{
    MANGA_PIPELINE_MAX_TOKENS_PROFILE, MangaLlmCallInput, MangaPromptGenerationTrace,
};
use super::checkpoint::load_manga_restored_llm_checkpoint;
use super::request::prepare_manga_llm_call;
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};

pub(in crate::manga) struct MangaRenderTraceInput<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub prompt_name: &'a str,
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub call_key: String,
    pub call_order: i32,
    pub restore_from_checkpoint: bool,
    pub variables: &'a HashMap<String, Value>,
    pub manifest: LlmContextManifest,
    pub temperature: f32,
}

impl OrchestratorContext {
    pub(in crate::manga) async fn render_and_chat_trace(
        &self,
        input: MangaRenderTraceInput<'_>,
    ) -> Result<MangaPromptGenerationTrace, AppError> {
        let MangaRenderTraceInput {
            prompt_renderer,
            prompt_name,
            project_id,
            workflow_run_id,
            call_key,
            call_order,
            restore_from_checkpoint,
            variables,
            manifest,
            temperature,
        } = input;
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let prepared = prepare_manga_llm_call(MangaLlmCallInput {
            prompt_renderer,
            prompt_name,
            project_id,
            workflow_run_id: Some(workflow_run_id),
            call_key,
            call_order,
            variables,
            manifest,
            temperature,
        })
        .await?;
        let repair_request = prepared.request.clone();
        let (response_text, usage, finish_reason, restored_parsed_output) =
            if restore_from_checkpoint
                && let Some(restored) = load_manga_restored_llm_checkpoint(
                    &self.db,
                    workflow_run_id,
                    &prepared.call_key,
                    &prepared.input_fingerprint_json,
                    prompt_name,
                )
                .await?
            {
                (
                    restored.response_text,
                    restored.usage,
                    restored.finish_reason,
                    Some(restored.parsed_output),
                )
            } else {
                let response = self.llm.chat(prepared.request).await?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err(AppError::InvalidArgument(format!(
                        "{prompt_name} 输出被 max_tokens 截断，未形成完整 JSON。当前使用 token 档位 {MANGA_PIPELINE_MAX_TOKENS_PROFILE}，请减少本章输入规模或增大该档位后重试。"
                    )));
                }
                (
                    response.message.content,
                    response.usage,
                    response.finish_reason,
                    None,
                )
            };
        Ok(MangaPromptGenerationTrace {
            prompt_name: prompt_name.to_string(),
            request_json: prepared.request_json,
            messages_json: prepared.messages_json,
            response_text,
            workflow_run_id: Some(workflow_run_id),
            call_key: Some(prepared.call_key),
            call_order: Some(prepared.call_order),
            input_fingerprint_json: Some(prepared.input_fingerprint_json),
            usage,
            finish_reason,
            context_manifest_summary_json: prepared.context_manifest_summary_json,
            restored_parsed_output,
            repair_request: Some(repair_request),
        })
    }

    pub(in crate::manga) async fn repair_and_validate_manga_llm_output<F>(
        &self,
        trace: &MangaPromptGenerationTrace,
        validate: F,
    ) -> Result<Value, AppError>
    where
        F: Fn(&Value) -> Result<Value, AppError>,
    {
        if let Some(restored) = trace.restored_parsed_output.as_ref() {
            return validate(restored);
        }
        let request = trace
            .repair_request
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_manga_repair_request".to_string(),
                operation: format!("修复漫画结构化输出 {}", trace.prompt_name),
            })?;
        let workflow_run_id = workflow::required_llm_call_workflow_run_id(
            trace.workflow_run_id,
            trace.call_key.as_deref().unwrap_or(&trace.prompt_name),
        )
        .map_err(workflow_checkpoint_contract_app_error)?;
        let call_key = trace
            .call_key
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_manga_call_key".to_string(),
                operation: format!("修复漫画结构化输出 {}", trace.prompt_name),
            })?;
        let call_order = trace.call_order.ok_or_else(|| AppError::InvalidState {
            current: "missing_manga_call_order".to_string(),
            operation: format!("修复漫画结构化输出 {}", trace.prompt_name),
        })?;
        let repaired = self
            .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
                request,
                raw_response: &trace.response_text,
                label: &trace.prompt_name,
                schema: crate::manga::pipeline_json::manga_llm_output_schema(&trace.prompt_name)?,
                checkpoint: Some(StructuredRepairCheckpointContext {
                    workflow_run_id,
                    base_call_key: call_key,
                    call_order,
                }),
                cancel_token: None,
                progress: None,
                parse_output: &validate,
            })
            .await?;
        Ok(repaired.output)
    }
}
