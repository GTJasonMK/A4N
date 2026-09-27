use a4n_error::AppError;
use serde::Serialize;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput},
};

impl OrchestratorContext {
    pub(super) async fn persist_raw_shooting_call(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        call_order: i32,
        fingerprint_json: Value,
        response_content: &str,
        usage: Option<Value>,
        finish_reason: Option<String>,
    ) -> Result<(), AppError> {
        let mut artifacts = serde_json::Map::new();
        artifacts.insert(
            super::RAW_RESPONSE_ONLY_ARTIFACT.to_string(),
            Value::Bool(true),
        );
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: call_key.to_string(),
            call_order,
            input_fingerprint_json: fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: Value::Bool(true),
                response_content: response_content.to_string(),
                usage,
                finish_reason,
                artifacts_json: Some(Value::Object(artifacts)),
            },
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
    ) -> Result<(), AppError> {
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: call_key.to_string(),
            call_order,
            input_fingerprint_json: fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: serde_json::to_value(output)?,
                response_content: response_content.to_string(),
                usage,
                finish_reason,
                artifacts_json: None,
            },
        })
        .await
    }
}
