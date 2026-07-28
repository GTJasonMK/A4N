use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::{Value, json};

use crate::workflow::{workflow_checkpoint_contract_app_error, workflow_write_app_error};

use super::super::super::OrchestratorContext;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;

impl OrchestratorContext {
    pub(in crate::manga::pipeline_run) async fn save_or_validate_manga_llm_checkpoint(
        &self,
        trace: &MangaPromptGenerationTrace,
        parsed_output: &Value,
    ) -> Result<(), AppError> {
        if let Some(restored) = trace.restored_parsed_output.as_ref() {
            if restored != parsed_output {
                let call_key = trace.call_key.as_deref().ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "漫画 LLM trace 缺少调用级 call_key: {}",
                        trace.prompt_name
                    ))
                })?;
                return Err(AppError::InvalidArgument(format!(
                    "{} checkpoint 的 parsed_output 未通过当前领域校验或规范化后发生变化",
                    call_key
                )));
            }
            return Ok(());
        }
        let call_key = trace.call_key.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 call_key: {}",
                trace.prompt_name
            ))
        })?;
        let call_order = trace.call_order.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 call_order: {}",
                trace.prompt_name
            ))
        })?;
        let input_fingerprint_json = trace.input_fingerprint_json.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 input_fingerprint_json: {}",
                trace.prompt_name
            ))
        })?;
        let response_usage_json = trace
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;
        workflow::save_completed_llm_call_checkpoint(
            self.database.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id: workflow::required_llm_call_workflow_run_id(
                    trace.workflow_run_id,
                    &call_key,
                )
                .map_err(workflow_checkpoint_contract_app_error)?,
                call_key,
                call_order,
                input_fingerprint_json,
                checkpoint_json: json!({
                    "output": parsed_output,
                    "parsed_output": parsed_output,
                    "response_content": trace.response_text,
                    "usage": response_usage_json,
                    "finish_reason": trace.finish_reason,
                }),
                artifacts_json: None,
            },
        )
        .await
        .map_err(workflow_write_app_error)
    }
}
