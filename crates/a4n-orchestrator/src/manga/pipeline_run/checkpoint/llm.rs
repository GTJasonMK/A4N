use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::{Value, json};

use crate::workflow::workflow_write_app_error;

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
                return Err(AppError::InvalidArgument(format!(
                    "{} checkpoint 的 parsed_output 与严格解析结果不一致",
                    trace.call_key
                )));
            }
            return Ok(());
        }
        let response_usage_json = trace
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;
        workflow::save_completed_llm_call_checkpoint(
            self.database.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id: trace.workflow_run_id,
                call_key: trace.call_key.clone(),
                call_order: trace.call_order,
                input_fingerprint_json: trace.input_fingerprint_json.clone(),
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
