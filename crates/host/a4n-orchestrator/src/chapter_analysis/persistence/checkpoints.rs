use a4n_db::workflow::{
    CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput, UpsertWorkflowCheckpointInput,
};
use a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::OrchestratorContext;

impl OrchestratorContext {
    pub(in crate::chapter_analysis) async fn save_completed_chapter_analysis_stage_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        input_fingerprint_json: Value,
        response_content: String,
        parsed_output: Value,
    ) -> Result<(), AppError> {
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: step_key.to_string(),
            call_order: step_order,
            input_fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output,
                response_content,
                usage: None,
                finish_reason: None,
                artifacts_json: None,
            },
        })
        .await
    }

    pub(in crate::chapter_analysis) async fn save_completed_chapter_analysis_final_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        input_fingerprint_json: Value,
        output: Value,
    ) -> Result<(), AppError> {
        let input_fingerprint_sha256 =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        self.upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key: step_key.to_string(),
            step_order,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input_fingerprint_json),
            checkpoint_json: Some(json!({ "output": output })),
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await?;
        Ok(())
    }
}
