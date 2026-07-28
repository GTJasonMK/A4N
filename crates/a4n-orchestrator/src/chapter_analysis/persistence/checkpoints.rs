use a4n_core::error::AppError;
use a4n_db::workflow::{
    self, CompletedLlmCallCheckpointInput, UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED,
};
use serde_json::{Value, json};

use super::super::OrchestratorContext;
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

impl OrchestratorContext {
    pub(in crate::chapter_analysis) async fn load_completed_chapter_analysis_stage_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_json: &Value,
    ) -> Result<Option<Value>, AppError> {
        workflow::load_completed_llm_call_checkpoint(
            self.database.connection(),
            workflow_run_id,
            step_key,
            input_fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)?
        .map(|checkpoint| {
            workflow::required_llm_call_checkpoint_json(checkpoint, step_key)
                .map_err(workflow_checkpoint_contract_app_error)
        })
        .transpose()
    }

    pub(in crate::chapter_analysis) async fn save_completed_chapter_analysis_stage_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        input_fingerprint_json: Value,
        response_content: String,
        parsed_output: Value,
    ) -> Result<(), AppError> {
        workflow::save_completed_llm_call_checkpoint(
            self.database.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: step_key.to_string(),
                call_order: step_order,
                input_fingerprint_json,
                checkpoint_json: json!({
                    "output": parsed_output.clone(),
                    "parsed_output": parsed_output,
                    "response_content": response_content,
                    "usage": null,
                    "finish_reason": null,
                }),
                artifacts_json: None,
            },
        )
        .await
        .map_err(workflow_write_app_error)
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
        workflow::upsert_workflow_checkpoint(
            self.database.connection(),
            UpsertWorkflowCheckpointInput {
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
            },
        )
        .await
        .map_err(workflow_write_app_error)?;
        Ok(())
    }

    pub(in crate::chapter_analysis) async fn mark_chapter_analysis_current_step(
        &self,
        workflow_run_id: i64,
        step_key: &str,
    ) -> Result<(), AppError> {
        workflow::mark_workflow_current_step(self.database.connection(), workflow_run_id, step_key)
            .await
            .map_err(workflow_write_app_error)?;
        Ok(())
    }
}
