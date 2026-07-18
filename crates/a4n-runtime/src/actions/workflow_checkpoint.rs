use a4n_core::error::AppError;
use a4n_core::models::generation::workflow_checkpoint;
use a4n_orchestrator::workflow::{self as orchestrator_workflow, WorkflowCheckpointContractError};
use serde_json::Value;

pub(crate) use a4n_orchestrator::workflow::CompletedLlmCallCheckpointInput;

pub(crate) fn llm_call_checkpoint_fingerprint_sha256(value: &Value) -> Result<String, AppError> {
    orchestrator_workflow::llm_call_checkpoint_fingerprint_sha256(value)
        .map_err(llm_checkpoint_contract_error)
}

pub(crate) fn required_llm_call_workflow_run_id(
    workflow_run_id: Option<i64>,
    call_key: &str,
) -> Result<i64, AppError> {
    orchestrator_workflow::required_llm_call_workflow_run_id(workflow_run_id, call_key)
        .map_err(llm_checkpoint_contract_error)
}

pub(crate) fn required_llm_call_checkpoint_json(
    checkpoint: workflow_checkpoint::Model,
    call_key: &str,
) -> Result<Value, AppError> {
    orchestrator_workflow::required_llm_call_checkpoint_json(checkpoint, call_key)
        .map_err(llm_checkpoint_contract_error)
}

fn llm_checkpoint_contract_error(error: WorkflowCheckpointContractError) -> AppError {
    match error {
        WorkflowCheckpointContractError::Invalid(message) => AppError::InvalidArgument(message),
        WorkflowCheckpointContractError::MissingCheckpointJson { call_key } => {
            AppError::InvalidState {
                current: "missing_checkpoint_json".to_string(),
                operation: format!("读取 LLM 调用断点 {call_key}"),
            }
        }
        WorkflowCheckpointContractError::Serialization(error) => {
            AppError::InvalidArgument(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_checkpoint_missing_payload_maps_to_invalid_state() {
        let error =
            llm_checkpoint_contract_error(WorkflowCheckpointContractError::MissingCheckpointJson {
                call_key: "chapter.scene".to_string(),
            });

        assert!(matches!(
            error,
            AppError::InvalidState { current, operation }
                if current == "missing_checkpoint_json"
                    && operation.contains("chapter.scene")
        ));
    }

    #[test]
    fn llm_checkpoint_invalid_contract_maps_to_invalid_argument() {
        let error = llm_checkpoint_contract_error(WorkflowCheckpointContractError::Invalid(
            "checkpoint 缺少 parsed_output".to_string(),
        ));

        assert!(matches!(
            error,
            AppError::InvalidArgument(message) if message.contains("parsed_output")
        ));
    }
}
