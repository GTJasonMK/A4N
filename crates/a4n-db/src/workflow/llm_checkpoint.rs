use super::*;

#[derive(Debug, Clone)]
pub struct CompletedLlmCallCheckpointInput {
    pub workflow_run_id: i64,
    pub call_key: String,
    pub call_order: i32,
    pub input_fingerprint_json: Value,
    pub checkpoint_json: Value,
    pub artifacts_json: Option<Value>,
}

pub fn llm_call_checkpoint_fingerprint_sha256(
    value: &Value,
) -> Result<String, WorkflowCheckpointContractError> {
    crate::generation::generation_checkpoint_fingerprint_sha256(value)
        .map_err(WorkflowCheckpointContractError::Serialization)
}

pub fn required_llm_call_workflow_run_id(
    workflow_run_id: Option<i64>,
    call_key: &str,
) -> Result<i64, WorkflowCheckpointContractError> {
    workflow_run_id.ok_or_else(|| {
        WorkflowCheckpointContractError::Invalid(format!(
            "执行 LLM 调用 {call_key} 需要 workflow_run_id"
        ))
    })
}

fn is_truncated_finish_reason(finish_reason: Option<&str>) -> bool {
    matches!(finish_reason, Some("length" | "max_tokens"))
}

fn has_restorable_llm_call_result(checkpoint_json: &Value) -> bool {
    checkpoint_json
        .get("parsed_output")
        .is_some_and(|value| !value.is_null())
        || checkpoint_json
            .get("response_content")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
}

pub fn validate_completed_llm_call_checkpoint_json(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(), WorkflowCheckpointContractError> {
    let object = checkpoint_json.as_object().ok_or_else(|| {
        WorkflowCheckpointContractError::Invalid(format!("{call_key} LLM 调用断点必须是对象"))
    })?;
    for field in [
        "output",
        "parsed_output",
        "response_content",
        "usage",
        "finish_reason",
    ] {
        if !object.contains_key(field) {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点缺少必需字段 {field}"
            )));
        }
    }
    if !matches!(object.get("response_content"), Some(Value::String(_))) {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点 response_content 必须是字符串"
        )));
    }
    let finish_reason = match object.get("finish_reason") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value.as_str()),
        _ => {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点 finish_reason 必须是字符串或 null"
            )));
        }
    };
    if is_truncated_finish_reason(finish_reason) {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点标记为截断响应，不能保存或恢复"
        )));
    }
    if !has_restorable_llm_call_result(checkpoint_json) {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点没有可恢复的响应内容"
        )));
    }
    Ok(())
}

pub fn required_llm_call_checkpoint_json(
    checkpoint: workflow_checkpoint::Model,
    call_key: &str,
) -> Result<Value, WorkflowCheckpointContractError> {
    let checkpoint_json = checkpoint.checkpoint_json.ok_or_else(|| {
        WorkflowCheckpointContractError::MissingCheckpointJson {
            call_key: call_key.to_string(),
        }
    })?;
    validate_completed_llm_call_checkpoint_json(&checkpoint_json, call_key)?;
    Ok(checkpoint_json)
}

pub async fn load_completed_llm_call_checkpoint(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<workflow_checkpoint::Model>, WorkflowReadError> {
    let input_fingerprint_sha256 =
        llm_call_checkpoint_fingerprint_sha256(input_fingerprint_json)
            .map_err(|error| WorkflowReadError::InvalidData(error.to_string()))?;
    let checkpoint = completed_workflow_checkpoint_by_fingerprint(
        db,
        workflow_run_id,
        call_key,
        &input_fingerprint_sha256,
    )
    .await?;
    Ok(checkpoint.filter(|checkpoint| {
        checkpoint
            .checkpoint_json
            .as_ref()
            .is_none_or(has_restorable_llm_call_result)
    }))
}

pub async fn save_completed_llm_call_checkpoint(
    db: &DatabaseConnection,
    input: CompletedLlmCallCheckpointInput,
) -> Result<(), WorkflowWriteError> {
    validate_completed_llm_call_checkpoint_json(&input.checkpoint_json, &input.call_key)
        .map_err(|error| WorkflowWriteError::InvalidArgument(error.to_string()))?;
    let input_fingerprint_sha256 =
        llm_call_checkpoint_fingerprint_sha256(&input.input_fingerprint_json)
            .map_err(|error| WorkflowWriteError::InvalidData(error.to_string()))?;
    upsert_workflow_checkpoint(
        db,
        UpsertWorkflowCheckpointInput {
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
        },
    )
    .await?;
    Ok(())
}
