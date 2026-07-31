use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::GenerationUsage;
use serde_json::{Value, json};

use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

use super::validation::{
    inspiration_call_fingerprint_json, required_checkpoint_string,
    validate_inspiration_response_json,
};
use crate::OrchestratorContext;

pub(super) struct InspirationCheckpointRequest {
    pub(super) request: ChatRequest,
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) fingerprint_json: Value,
}

impl OrchestratorContext {
    pub(super) async fn prepare_inspiration_checkpoint_request(
        &self,
        project_id: i64,
        mut request: ChatRequest,
        workflow_run_id: Option<i64>,
    ) -> Result<InspirationCheckpointRequest, AppError> {
        let call_key = format!("inspiration_conversation.project_{project_id}.turn");
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let fingerprint_json = inspiration_call_fingerprint_json(project_id, &call_key, &request)?;
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )?;
        workflow::mark_workflow_current_step(
            self.database.connection(),
            workflow_run_id,
            &call_key,
        )
        .await
        .map_err(workflow_write_app_error)?;
        Ok(InspirationCheckpointRequest {
            request,
            workflow_run_id,
            call_key,
            fingerprint_json,
        })
    }

    pub(super) async fn load_inspiration_checkpoint(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        fingerprint_json: &Value,
    ) -> Result<Option<String>, AppError> {
        let call_key = format!("inspiration_conversation.project_{project_id}.turn");
        let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
            self.database.connection(),
            workflow_run_id,
            &call_key,
            fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)?
        else {
            return Ok(None);
        };
        let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
        let parsed_output =
            required_checkpoint_string(&checkpoint_json, &call_key, "parsed_output")?;
        let response_content =
            required_checkpoint_string(&checkpoint_json, &call_key, "response_content")?;
        let reparsed = validate_inspiration_response_json(&response_content)?;
        if reparsed != parsed_output {
            return Err(AppError::InvalidState {
                current: "checkpoint_parsed_output_mismatch".to_string(),
                operation: format!("恢复灵感对话 LLM 调用断点 {call_key}"),
            });
        }
        Ok(Some(reparsed))
    }

    pub(super) async fn save_inspiration_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: String,
        fingerprint_json: Value,
        output: &str,
        finish_reason: Option<String>,
        usage: Option<GenerationUsage>,
    ) -> Result<(), AppError> {
        workflow::save_completed_llm_call_checkpoint(
            self.database.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": output,
                    "parsed_output": output,
                    "response_content": output,
                    "finish_reason": finish_reason,
                    "usage": usage,
                }),
                artifacts_json: None,
            },
        )
        .await
        .map_err(workflow_write_app_error)
    }
}
