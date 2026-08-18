use a4n_core::error::AppError;
use serde_json::Value;

use crate::{
    structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput},
};

use super::super::super::OrchestratorContext;
use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;

impl OrchestratorContext {
    pub(in crate::manga::pipeline_run) async fn save_or_validate_manga_llm_checkpoint(
        &self,
        trace: &MangaPromptGenerationTrace,
        parsed_output: &Value,
    ) -> Result<(), AppError> {
        if let Some(restored) = trace.restored_parsed_output.as_ref() {
            ensure_checkpoint_parsed_output_matches(
                restored,
                parsed_output,
                &format!("漫画管线 {}", trace.call_key),
            )?;
            return Ok(());
        }
        let response_usage_json = trace
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: trace.workflow_run_id,
                call_key: trace.call_key.clone(),
                call_order: trace.call_order,
                input_fingerprint_json: trace.input_fingerprint_json.clone(),
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: parsed_output.clone(),
                    response_content: trace.response_text.clone(),
                    usage: response_usage_json,
                    finish_reason: trace.finish_reason.clone(),
                    artifacts_json: None,
                },
        })
        .await
    }
}
