use super::{WorkflowCheckpointContractError, WorkflowReadError, WorkflowWriteError};
use a4n_core::error::AppError;

pub(crate) fn workflow_checkpoint_contract_app_error(
    error: WorkflowCheckpointContractError,
) -> AppError {
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

pub(crate) fn workflow_read_app_error(error: WorkflowReadError) -> AppError {
    match error {
        WorkflowReadError::InvalidArgument(message) => AppError::InvalidArgument(message),
        WorkflowReadError::NotFound(message) => AppError::NotFound(message),
        WorkflowReadError::InvalidData(message) => AppError::Internal(anyhow::anyhow!(message)),
        WorkflowReadError::Database(error) => AppError::Database(error),
    }
}

pub(crate) fn workflow_write_app_error(error: WorkflowWriteError) -> AppError {
    match error {
        WorkflowWriteError::InvalidArgument(message) => AppError::InvalidArgument(message),
        WorkflowWriteError::NotFound(message) => AppError::NotFound(message),
        WorkflowWriteError::InvalidData(message) => AppError::Internal(anyhow::anyhow!(message)),
        WorkflowWriteError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_missing_json_maps_to_invalid_state() {
        let error = workflow_checkpoint_contract_app_error(
            WorkflowCheckpointContractError::MissingCheckpointJson {
                call_key: "chapter_analysis.facts".to_string(),
            },
        );

        match error {
            AppError::InvalidState { current, operation } => {
                assert_eq!(current, "missing_checkpoint_json");
                assert_eq!(operation, "读取 LLM 调用断点 chapter_analysis.facts");
            }
            other => panic!("expected invalid state, got {other}"),
        }
    }

    #[test]
    fn workflow_read_invalid_argument_preserves_message() {
        let error = workflow_read_app_error(WorkflowReadError::InvalidArgument(
            "workflow_run_id 必须大于 0".to_string(),
        ));

        match error {
            AppError::InvalidArgument(message) => {
                assert_eq!(message, "workflow_run_id 必须大于 0");
            }
            other => panic!("expected invalid argument, got {other}"),
        }
    }

    #[test]
    fn workflow_write_not_found_preserves_message() {
        let error = workflow_write_app_error(WorkflowWriteError::NotFound(
            "工作流不存在: workflow_run_id=7".to_string(),
        ));

        match error {
            AppError::NotFound(message) => {
                assert_eq!(message, "工作流不存在: workflow_run_id=7");
            }
            other => panic!("expected not found, got {other}"),
        }
    }
}
