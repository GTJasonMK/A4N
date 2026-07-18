use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use a4n_llm::provider::{ChatRequest, is_truncated_finish_reason};
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

pub mod design_brief;

mod prompt_generation;
mod types;

pub use prompt_generation::{
    PortraitPromptGenerationInput, PortraitPromptRequestInput, PortraitPromptSpec,
    build_generate_prompt_request, normalize_portrait_prompt_response,
};

#[cfg(test)]
mod tests;

impl OrchestratorContext {
    pub async fn build_generate_prompt_request(
        &self,
        input: PortraitPromptRequestInput<'_>,
    ) -> Result<ChatRequest, AppError> {
        build_generate_prompt_request(input).await
    }

    pub async fn generate_prompt_with_checkpoint(
        &self,
        input: PortraitPromptGenerationInput<'_>,
    ) -> Result<String, AppError> {
        let PortraitPromptGenerationInput {
            project_id,
            spec,
            prompt_renderer,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_generate_prompt_request(PortraitPromptRequestInput {
            project_id,
            spec,
            prompt_renderer,
            variables,
        })
        .await?;

        let call_key = "portrait_prompt.generate".to_string();
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let fingerprint_json = portrait_prompt_call_fingerprint_json(
            project_id,
            spec.profile_name,
            spec.portrait_type,
            &request,
        )?;
        let input_fingerprint_sha256 =
            workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }

        workflow::mark_workflow_current_step(self.db.connection(), workflow_run_id, &call_key)
            .await
            .map_err(workflow_write_app_error)?;
        if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
            self.db.connection(),
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)?
        {
            let checkpoint_json =
                workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
            return restored_portrait_prompt_checkpoint(checkpoint_json, &call_key);
        }

        let response = self.llm.chat(request).await?;
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
        workflow::save_completed_llm_call_checkpoint(
            self.db.connection(),
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
        .await
        .map_err(workflow_write_app_error)?;
        Ok(output)
    }
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
        "request": a4n_llm::trace::context_aware_chat_request_trace_json(request)?,
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
