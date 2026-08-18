use a4n_core::error::AppError;
use a4n_db::workflow::{
    self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput,
};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::GenerationUsage;
use serde_json::Value;

use crate::{
    structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::workflow_checkpoint_contract_app_error,
};

use super::validation::{
    inspiration_call_fingerprint_json, validate_inspiration_response_json,
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
        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
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
        let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, fingerprint_json)
            .await?
        else {
            return Ok(None);
        };
        let parsed_output = checkpoint.parsed_output.as_str().ok_or_else(|| {
            AppError::InvalidState {
                current: "invalid_checkpoint_field:parsed_output".to_string(),
                operation: format!("读取灵感对话 LLM 调用断点 {call_key}"),
            }
        })?;
        let reparsed = validate_inspiration_response_json(&checkpoint.response_content)?;
        ensure_checkpoint_parsed_output_matches(
            parsed_output,
            &reparsed,
            &format!("灵感对话 {call_key}"),
        )?;
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
        let usage = usage.map(serde_json::to_value).transpose()?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json: fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: Value::String(output.to_string()),
                    response_content: output.to_string(),
                    usage,
                    finish_reason,
                    artifacts_json: None,
                },
        })
        .await
    }
}
