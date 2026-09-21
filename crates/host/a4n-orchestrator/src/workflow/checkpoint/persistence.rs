use super::super::{UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED};
use super::contracts::{
    CompletedWorkflowCheckpointContractError, CompletedWorkflowCheckpointInput,
    workflow_checkpoint_fingerprint_sha256,
};
use crate::OrchestratorContext;
use a4n_db_models::models::generation::workflow_checkpoint;
use a4n_error::AppError;
use serde_json::Value;

impl OrchestratorContext {
    pub async fn load_completed_workflow_checkpoint(
        &self,
        workflow_run_id: Option<i64>,
        step_key: &str,
        input_fingerprint_json: &Value,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        let Some(workflow_run_id) = workflow_run_id else {
            return Ok(None);
        };
        let fingerprint_sha256 = workflow_checkpoint_fingerprint_sha256(input_fingerprint_json)
            .map_err(completed_workflow_checkpoint_contract_app_error)?;
        self.completed_workflow_checkpoint_by_fingerprint(
            workflow_run_id,
            step_key,
            &fingerprint_sha256,
        )
        .await
    }

    pub async fn save_completed_workflow_checkpoint(
        &self,
        input: CompletedWorkflowCheckpointInput,
    ) -> Result<(), AppError> {
        let Some(input) = completed_workflow_checkpoint_upsert_input(input)
            .map_err(completed_workflow_checkpoint_contract_app_error)?
        else {
            return Ok(());
        };
        self.upsert_workflow_checkpoint(input).await?;
        Ok(())
    }
}

pub(super) fn completed_workflow_checkpoint_upsert_input(
    input: CompletedWorkflowCheckpointInput,
) -> Result<Option<UpsertWorkflowCheckpointInput>, CompletedWorkflowCheckpointContractError> {
    let Some(workflow_run_id) = input.workflow_run_id else {
        return Ok(None);
    };
    let input_fingerprint_sha256 =
        workflow_checkpoint_fingerprint_sha256(&input.input_fingerprint_json)?;
    Ok(Some(UpsertWorkflowCheckpointInput {
        workflow_run_id,
        step_key: input.step_key,
        step_order: input.step_order,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        input_fingerprint_sha256: Some(input_fingerprint_sha256),
        input_fingerprint_json: Some(input.input_fingerprint_json),
        checkpoint_json: input.checkpoint_json,
        artifacts_json: None,
        generation_run_ids_json: None,
        error_message: None,
        completed: true,
    }))
}

fn completed_workflow_checkpoint_contract_app_error(
    error: CompletedWorkflowCheckpointContractError,
) -> AppError {
    AppError::InvalidArgument(match error {
        CompletedWorkflowCheckpointContractError::Serialization(error) => error.to_string(),
        error => error.to_string(),
    })
}
