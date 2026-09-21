use a4n_db_models::models::generation::workflow_checkpoint;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum CompletedWorkflowCheckpointContractError {
    #[error("{step_key} 已完成 checkpoint 缺少 checkpoint_json")]
    MissingCheckpointJson { step_key: String },
    #[error("{step_key} 已完成 checkpoint 缺少 {field}")]
    MissingCheckpointField { step_key: String, field: String },
    #[error("工作流 checkpoint 指纹序列化失败: {0}")]
    Serialization(#[source] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct CompletedWorkflowCheckpointInput {
    pub workflow_run_id: Option<i64>,
    pub step_key: String,
    pub step_order: i32,
    pub input_fingerprint_json: Value,
    pub checkpoint_json: Option<Value>,
}

pub fn workflow_checkpoint_fingerprint_sha256(
    value: &Value,
) -> Result<String, CompletedWorkflowCheckpointContractError> {
    a4n_db::generation::generation_checkpoint_fingerprint_sha256(value)
        .map_err(CompletedWorkflowCheckpointContractError::Serialization)
}

pub fn required_checkpoint_json(
    checkpoint: &workflow_checkpoint::Model,
    step_key: &str,
) -> Result<Value, CompletedWorkflowCheckpointContractError> {
    checkpoint.checkpoint_json.clone().ok_or_else(|| {
        CompletedWorkflowCheckpointContractError::MissingCheckpointJson {
            step_key: step_key.to_string(),
        }
    })
}

pub fn required_checkpoint_field(
    checkpoint_json: &Value,
    step_key: &str,
    field: &str,
) -> Result<Value, CompletedWorkflowCheckpointContractError> {
    checkpoint_json.get(field).cloned().ok_or_else(|| {
        CompletedWorkflowCheckpointContractError::MissingCheckpointField {
            step_key: step_key.to_string(),
            field: field.to_string(),
        }
    })
}
