use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    structured_llm::{ChatJsonStreamInput, parse_strict_json_response, stream_error_or_cancelled},
    workflow::{
        CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
        workflow_checkpoint_contract_app_error,
    },
};

use super::{
    checkpoint::{CompatibleRepairCheckpointInput, STRUCTURED_REPAIR_CHECKPOINT_SCHEMA_VERSION},
    contracts::StructuredValueRepairInput,
    rules::report_progress,
};

impl OrchestratorContext {
    pub(super) async fn run_structured_repair_call<T, P>(
        &self,
        input: &StructuredValueRepairInput<'_, P>,
        suffix: &str,
        request: ChatRequest,
        target_schema_sha256: &str,
        artifact_details: Option<Value>,
    ) -> Result<RepairCallResult, AppError>
    where
        P: Fn(&Value) -> Result<T, AppError>,
    {
        let call_key = input
            .checkpoint
            .as_ref()
            .map(|checkpoint| format!("{}:{suffix}", checkpoint.base_call_key))
            .or_else(|| {
                request
                    .trace_context
                    .as_ref()
                    .and_then(|trace| trace.call_key.as_deref())
                    .map(|base_call_key| format!("{base_call_key}:{suffix}"))
            })
            .unwrap_or_else(|| format!("structured_output:{suffix}"));
        let mut request = request;
        if let Some(trace) = request.trace_context.as_mut() {
            trace.call_key = Some(call_key.clone());
        }
        let fingerprint_json = a4n_llm::trace::context_aware_chat_request_trace_json(&request)?;
        let base_response_sha256 = a4n_core::source_spans::source_text_hash(input.raw_response);

        if let Some(checkpoint_context) = input.checkpoint.as_ref() {
            if let Some(checkpoint) = self
                .load_completed_llm_call_checkpoint(
                    checkpoint_context.workflow_run_id,
                    &call_key,
                    &fingerprint_json,
                )
                .await?
            {
                let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
                let value = checkpoint_json
                    .get("parsed_output")
                    .filter(|value| !value.is_null())
                    .cloned()
                    .ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "{call_key}.parsed_output 缺失，不能恢复结构纠正断点"
                        ))
                    })?;
                if validate_repair_value(suffix, &value, input.parse_output).is_ok() {
                    return Ok(RepairCallResult {
                        value,
                        restored: true,
                    });
                }
            }
            if let Some(value) = self
                .load_compatible_structured_repair_value(CompatibleRepairCheckpointInput {
                    workflow_run_id: checkpoint_context.workflow_run_id,
                    base_call_key: &checkpoint_context.base_call_key,
                    call_key: &call_key,
                    suffix,
                    base_response_sha256: &base_response_sha256,
                    target_schema_sha256,
                    require_target_validation: suffix == "schema_repair",
                    parse_output: input.parse_output,
                })
                .await?
            {
                return Ok(RepairCallResult {
                    value,
                    restored: true,
                });
            }
            self.mark_workflow_current_step(checkpoint_context.workflow_run_id, &call_key)
                .await?;
        }

        let result = self
            .chat_json_stream_result_with_progress(
                ChatJsonStreamInput {
                    request,
                    prompt_name: &call_key,
                    truncated_message: "结构纠正输出达到 max_tokens 上限，JSON 未完整闭合",
                    structured_progress: None,
                    cancel_token: input.cancel_token,
                    cancelled_message: "结构纠正已停止",
                },
                |_current, message| {
                    report_progress(input.progress, message).map_err(|error| error.to_string())
                },
            )
            .await
            .map_err(|error| stream_error_or_cancelled(error, input.cancel_token))?;
        let value: Value = parse_strict_json_response(&result.response_content, input.label)?;
        validate_repair_value(suffix, &value, input.parse_output)?;

        if let Some(checkpoint_context) = input.checkpoint.as_ref() {
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: checkpoint_context.workflow_run_id,
                call_key: call_key.clone(),
                call_order: checkpoint_context.call_order,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": &value,
                    "parsed_output": &value,
                    "response_content": result.response_content,
                    "usage": result.usage,
                    "finish_reason": result.finish_reason,
                }),
                artifacts_json: Some(json!({
                    "structured_repair": {
                        "schema_version": STRUCTURED_REPAIR_CHECKPOINT_SCHEMA_VERSION,
                        "mode": suffix,
                        "target_call_key": checkpoint_context.base_call_key,
                        "base_response_sha256": base_response_sha256,
                        "target_schema_sha256": target_schema_sha256,
                        "details": artifact_details,
                    }
                })),
            })
            .await?;
        }
        Ok(RepairCallResult {
            value,
            restored: false,
        })
    }
}

fn validate_repair_value<T, P>(
    suffix: &str,
    value: &Value,
    parse_output: &P,
) -> Result<(), AppError>
where
    P: Fn(&Value) -> Result<T, AppError>,
{
    if suffix == "schema_repair" {
        parse_output(value).map(|_| ())
    } else {
        Ok(())
    }
}

pub(super) struct RepairCallResult {
    pub(super) value: Value,
    pub(super) restored: bool,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn full_repair_values_must_match_the_target_contract() {
        let parse_output = |value: &Value| {
            value
                .get("required")
                .filter(|value| value.is_string())
                .map(|_| ())
                .ok_or_else(|| AppError::InvalidArgument("required 缺失".to_string()))
        };
        let invalid = json!({"unexpected": true});

        assert!(validate_repair_value("schema_repair", &invalid, &parse_output).is_err());
        assert!(validate_repair_value("schema_patch", &invalid, &parse_output).is_ok());
    }
}
