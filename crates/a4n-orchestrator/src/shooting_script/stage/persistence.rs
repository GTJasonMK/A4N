use a4n_core::error::AppError;
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext, structured_llm::StructuredLlmStreamResult,
    workflow::CompletedLlmCallCheckpointInput,
};

impl OrchestratorContext {
    pub(super) async fn persist_raw_shooting_call(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        call_order: i32,
        fingerprint_json: Value,
        result: &StructuredLlmStreamResult,
    ) -> Result<(), AppError> {
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: call_key.to_string(),
            call_order,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": result.response_content,
                "parsed_output": null,
                "response_content": result.response_content,
                "usage": result.usage,
                "finish_reason": result.finish_reason,
            }),
            artifacts_json: None,
        })
        .await
    }

    pub(super) async fn persist_parsed_shooting_call<T: Serialize>(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        call_order: i32,
        fingerprint_json: Value,
        response_content: &str,
        output: &T,
        usage: Option<Value>,
        finish_reason: Option<String>,
        artifacts_json: Option<Value>,
    ) -> Result<(), AppError> {
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: call_key.to_string(),
            call_order,
            input_fingerprint_json: fingerprint_json,
            checkpoint_json: json!({
                "output": output,
                "parsed_output": output,
                "response_content": response_content,
                "usage": usage,
                "finish_reason": finish_reason,
            }),
            artifacts_json,
        })
        .await
    }
}
