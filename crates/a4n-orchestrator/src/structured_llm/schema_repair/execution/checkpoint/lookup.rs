use a4n_core::error::AppError;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    workflow::{required_llm_call_checkpoint_json, workflow_checkpoint_contract_app_error},
};

use super::{
    artifact::compatible_structured_repair_artifact, contracts::CompatibleRepairCheckpointInput,
};

impl OrchestratorContext {
    pub(in crate::structured_llm::schema_repair::execution) async fn load_compatible_structured_repair_value<
        T,
        P,
    >(
        &self,
        input: CompatibleRepairCheckpointInput<'_, P>,
    ) -> Result<Option<Value>, AppError>
    where
        P: Fn(&Value) -> Result<T, AppError>,
    {
        let checkpoint = self
            .workflow_checkpoint_models(input.workflow_run_id)
            .await?
            .into_iter()
            .find(|checkpoint| {
                checkpoint.step_key == input.call_key
                    && checkpoint.status == crate::workflow::WORKFLOW_STATUS_COMPLETED
                    && compatible_structured_repair_artifact(
                        checkpoint.artifacts_json.as_ref(),
                        input.base_call_key,
                        input.suffix,
                        input.base_response_sha256,
                        input.target_schema_sha256,
                    )
            });
        let Some(checkpoint) = checkpoint else {
            return Ok(None);
        };
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, input.call_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
        let value = checkpoint_json
            .get("parsed_output")
            .filter(|value| !value.is_null())
            .cloned()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{}.parsed_output 缺失，不能恢复结构纠正断点",
                    input.call_key
                ))
            })?;
        if input.require_target_validation && (input.parse_output)(&value).is_err() {
            return Ok(None);
        }
        Ok(Some(value))
    }
}
