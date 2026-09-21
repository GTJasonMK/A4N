use super::*;

#[derive(Debug, Clone)]
pub struct CompletedLlmCallCheckpoint {
    pub parsed_output: Value,
    pub response_content: String,
    pub usage: Option<Value>,
    pub finish_reason: Option<String>,
    pub artifacts_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct CompletedLlmCallCheckpointInput {
    pub workflow_run_id: i64,
    pub call_key: String,
    pub call_order: i32,
    pub input_fingerprint_json: Value,
    pub checkpoint: CompletedLlmCallCheckpoint,
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

fn completed_llm_call_checkpoint_from_json(
    checkpoint_json: Value,
    artifacts_json: Option<Value>,
    call_key: &str,
) -> Result<CompletedLlmCallCheckpoint, WorkflowCheckpointContractError> {
    let mut object = match checkpoint_json {
        Value::Object(object) => object,
        _ => {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点必须是对象"
            )));
        }
    };
    const FIELDS: [&str; 4] = [
        "parsed_output",
        "response_content",
        "usage",
        "finish_reason",
    ];
    for field in FIELDS {
        if !object.contains_key(field) {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点缺少必需字段 {field}"
            )));
        }
    }
    if let Some(field) = object
        .keys()
        .find(|field| !FIELDS.contains(&field.as_str()))
    {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点包含未知字段 {field}"
        )));
    }
    let parsed_output = object.remove("parsed_output").ok_or_else(|| {
        WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点缺少必需字段 parsed_output"
        ))
    })?;
    if parsed_output.is_null() {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点 parsed_output 不能为 null"
        )));
    }
    let response_content = match object.remove("response_content") {
        Some(Value::String(value)) => value,
        _ => {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点 response_content 必须是字符串"
            )));
        }
    };
    if response_content.trim().is_empty() {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点 response_content 不能为空"
        )));
    }
    let usage = match object.remove("usage") {
        Some(Value::Null) => None,
        Some(value) => Some(value),
        None => {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点缺少必需字段 usage"
            )));
        }
    };
    let finish_reason = match object.remove("finish_reason") {
        Some(Value::Null) => None,
        Some(Value::String(value)) => Some(value),
        _ => {
            return Err(WorkflowCheckpointContractError::Invalid(format!(
                "{call_key} LLM 调用断点 finish_reason 必须是字符串或 null"
            )));
        }
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点标记为截断响应，不能保存或恢复"
        )));
    }
    Ok(CompletedLlmCallCheckpoint {
        parsed_output,
        response_content,
        usage,
        finish_reason,
        artifacts_json,
    })
}

pub(in crate::workflow) fn validate_completed_llm_call_checkpoint_json(
    checkpoint_json: &Value,
    call_key: &str,
) -> Result<(), WorkflowCheckpointContractError> {
    completed_llm_call_checkpoint_from_json(checkpoint_json.clone(), None, call_key).map(drop)
}

pub fn completed_llm_call_checkpoint_from_model(
    checkpoint: &workflow_checkpoint::Model,
    call_key: &str,
) -> Result<CompletedLlmCallCheckpoint, WorkflowCheckpointContractError> {
    let checkpoint_json = checkpoint.checkpoint_json.clone().ok_or_else(|| {
        WorkflowCheckpointContractError::Invalid(format!(
            "{call_key} LLM 调用断点缺少 checkpoint_json"
        ))
    })?;
    completed_llm_call_checkpoint_from_json(
        checkpoint_json,
        checkpoint.artifacts_json.clone(),
        call_key,
    )
}

pub async fn load_completed_llm_call_checkpoint(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<CompletedLlmCallCheckpoint>, WorkflowReadError> {
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
    checkpoint
        .map(|checkpoint| completed_llm_call_checkpoint_from_model(&checkpoint, call_key))
        .transpose()
        .map_err(|error| WorkflowReadError::InvalidData(error.to_string()))
}

pub(super) async fn inherit_completed_llm_call_checkpoints(
    tx: &sea_orm::DatabaseTransaction,
    source_run_id: i64,
    target_run_id: i64,
) -> Result<(), WorkflowWriteError> {
    let checkpoints = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(source_run_id))
        .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_COMPLETED))
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .order_by_asc(workflow_checkpoint::Column::Id)
        .all(tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    for checkpoint in checkpoints {
        let Some(value) = checkpoint.checkpoint_json.as_ref() else {
            continue;
        };
        // response_content 属于 LLM 调用断点合同，不继承业务写入或其他阶段的完成标记。
        if value.get("response_content").is_none() {
            continue;
        }
        validate_completed_llm_call_checkpoint_json(value, &checkpoint.step_key)
            .map_err(|error| WorkflowWriteError::InvalidData(error.to_string()))?;
        upsert_workflow_checkpoint_in_transaction(
            tx,
            UpsertWorkflowCheckpointInput {
                workflow_run_id: target_run_id,
                step_key: checkpoint.step_key,
                step_order: checkpoint.step_order,
                status: WORKFLOW_STATUS_COMPLETED.to_string(),
                input_fingerprint_sha256: checkpoint.input_fingerprint_sha256,
                input_fingerprint_json: checkpoint.input_fingerprint_json,
                checkpoint_json: checkpoint.checkpoint_json,
                artifacts_json: checkpoint.artifacts_json,
                generation_run_ids_json: checkpoint.generation_run_ids_json,
                error_message: None,
                completed: true,
            },
        )
        .await?;
    }
    Ok(())
}

pub async fn save_completed_llm_call_checkpoint(
    db: &DatabaseConnection,
    input: CompletedLlmCallCheckpointInput,
) -> Result<(), WorkflowWriteError> {
    let CompletedLlmCallCheckpoint {
        parsed_output,
        response_content,
        usage,
        finish_reason,
        artifacts_json,
    } = input.checkpoint;
    let checkpoint_json = json!({
        "parsed_output": parsed_output,
        "response_content": response_content,
        "usage": usage,
        "finish_reason": finish_reason,
    });
    validate_completed_llm_call_checkpoint_json(&checkpoint_json, &input.call_key)
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
            checkpoint_json: Some(checkpoint_json),
            artifacts_json,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        },
    )
    .await?;
    Ok(())
}
