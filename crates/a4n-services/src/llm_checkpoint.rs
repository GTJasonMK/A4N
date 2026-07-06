//! LLM call level workflow checkpoints.
//!
//! A workflow step may contain multiple model calls. Each resumable model call
//! must use a stable call key as `workflow_checkpoint.step_key`, and only write
//! a completed checkpoint after the response has passed the business parser and
//! validation for that call.

use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_checkpoint;
use a4n_engine::llm::provider::is_truncated_finish_reason;
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::generation::generation_checkpoint_fingerprint_sha256;
use crate::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, WorkflowRunService,
};

#[derive(Debug, Clone)]
pub struct CompletedLlmCallCheckpointInput {
    pub workflow_run_id: i64,
    pub call_key: String,
    pub call_order: i32,
    pub input_fingerprint_json: Value,
    pub checkpoint_json: Value,
    pub artifacts_json: Option<Value>,
}

pub fn llm_call_checkpoint_fingerprint_sha256(value: &Value) -> Result<String, AppError> {
    generation_checkpoint_fingerprint_sha256(value)
}

pub fn required_llm_call_workflow_run_id(
    workflow_run_id: Option<i64>,
    call_key: &str,
) -> Result<i64, AppError> {
    workflow_run_id.ok_or_else(|| {
        AppError::InvalidArgument(format!("执行 LLM 调用 {call_key} 需要 workflow_run_id"))
    })
}

pub async fn load_completed_llm_call_checkpoint(
    db: Arc<DatabaseConnection>,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<workflow_checkpoint::Model>, AppError> {
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(input_fingerprint_json)?;
    WorkflowRunService::new(db)
        .completed_checkpoint_by_fingerprint(workflow_run_id, call_key, &input_fingerprint_sha256)
        .await
}

pub fn required_llm_call_checkpoint_json(
    checkpoint: workflow_checkpoint::Model,
    call_key: &str,
) -> Result<Value, AppError> {
    let checkpoint_json = checkpoint
        .checkpoint_json
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_json".to_string(),
            operation: format!("读取 LLM 调用断点 {call_key}"),
        })?;
    validate_completed_llm_call_checkpoint_json(&checkpoint_json, call_key)?;
    Ok(checkpoint_json)
}

pub async fn save_completed_llm_call_checkpoint(
    db: Arc<DatabaseConnection>,
    input: CompletedLlmCallCheckpointInput,
) -> Result<(), AppError> {
    validate_completed_llm_call_checkpoint_json(&input.checkpoint_json, &input.call_key)?;
    let input_fingerprint_sha256 =
        llm_call_checkpoint_fingerprint_sha256(&input.input_fingerprint_json)?;
    WorkflowRunService::new(db)
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id: input.workflow_run_id,
            step_key: input.call_key,
            step_order: input.call_order,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input.input_fingerprint_json),
            checkpoint_json: Some(input.checkpoint_json),
            artifacts_json: input.artifacts_json,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await?;
    Ok(())
}

fn validate_completed_llm_call_checkpoint_json(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(), AppError> {
    let object = checkpoint_json
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key} LLM 调用断点必须是对象")))?;
    for field in [
        "output",
        "parsed_output",
        "response_content",
        "usage",
        "finish_reason",
    ] {
        if !object.contains_key(field) {
            return Err(AppError::InvalidArgument(format!(
                "{call_key} LLM 调用断点缺少必需字段 {field}"
            )));
        }
    }
    if !matches!(object.get("response_content"), Some(Value::String(_))) {
        return Err(AppError::InvalidArgument(format!(
            "{call_key} LLM 调用断点 response_content 必须是字符串"
        )));
    }
    let finish_reason = match object.get("finish_reason") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.as_str()),
        _ => {
            return Err(AppError::InvalidArgument(format!(
                "{call_key} LLM 调用断点 finish_reason 必须是字符串或 null"
            )));
        }
    };
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "{call_key} LLM 调用断点标记为截断响应，不能保存或恢复"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use super::*;

    #[test]
    fn validates_completed_llm_call_checkpoint_contract() {
        validate_completed_llm_call_checkpoint_json(
            &json!({
                "output": {"value": 1},
                "parsed_output": {"value": 1},
                "response_content": "{\"value\":1}",
                "usage": null,
                "finish_reason": null,
            }),
            "test.call",
        )
        .expect("valid checkpoint");
    }

    #[test]
    fn rejects_completed_llm_call_checkpoint_without_parsed_output() {
        let err = validate_completed_llm_call_checkpoint_json(
            &json!({
                "output": {"value": 1},
                "response_content": "{\"value\":1}",
                "usage": null,
                "finish_reason": null,
            }),
            "test.call",
        )
        .expect_err("missing parsed_output must fail");

        assert!(err.to_string().contains("parsed_output"));
    }

    #[test]
    fn rejects_completed_llm_call_checkpoint_with_non_string_response_content() {
        let err = validate_completed_llm_call_checkpoint_json(
            &json!({
                "output": {"value": 1},
                "parsed_output": {"value": 1},
                "response_content": {"value": 1},
                "usage": null,
                "finish_reason": null,
            }),
            "test.call",
        )
        .expect_err("non-string response_content must fail");

        assert!(err.to_string().contains("response_content"));
    }

    #[test]
    fn rejects_completed_llm_call_checkpoint_with_invalid_finish_reason() {
        let err = validate_completed_llm_call_checkpoint_json(
            &json!({
                "output": {"value": 1},
                "parsed_output": {"value": 1},
                "response_content": "{\"value\":1}",
                "usage": null,
                "finish_reason": {"reason": "stop"},
            }),
            "test.call",
        )
        .expect_err("finish_reason must be string or null");

        assert!(
            err.to_string()
                .contains("finish_reason 必须是字符串或 null")
        );
    }

    #[test]
    fn rejects_completed_llm_call_checkpoint_with_truncated_finish_reason() {
        let err = validate_completed_llm_call_checkpoint_json(
            &json!({
                "output": {"value": 1},
                "parsed_output": {"value": 1},
                "response_content": "{\"value\":1}",
                "usage": null,
                "finish_reason": "length",
            }),
            "test.call",
        )
        .expect_err("truncated response checkpoint must not be accepted");

        assert!(err.to_string().contains("标记为截断响应"));
    }

    #[test]
    fn required_llm_call_checkpoint_json_rejects_incomplete_loaded_checkpoint() {
        let now = Utc::now().naive_utc();
        let err = required_llm_call_checkpoint_json(
            workflow_checkpoint::Model {
                id: 1,
                workflow_run_id: 1,
                step_key: "test.call".to_string(),
                step_order: 1,
                status: WORKFLOW_STATUS_COMPLETED.to_string(),
                input_fingerprint_sha256: Some("fingerprint".to_string()),
                input_fingerprint_json: Some(json!({ "input": true })),
                checkpoint_json: Some(json!({
                    "output": {"value": 1},
                    "response_content": "{\"value\":1}",
                    "usage": null,
                    "finish_reason": null,
                })),
                artifacts_json: None,
                generation_run_ids_json: None,
                error_message: None,
                created_at: now,
                updated_at: now,
                completed_at: Some(now),
            },
            "test.call",
        )
        .expect_err("loaded checkpoint missing parsed_output must fail");

        assert!(err.to_string().contains("parsed_output"));
    }
}
