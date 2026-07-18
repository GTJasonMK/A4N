use a4n_core::error::AppError;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    OrchestratorContext, structured_llm::schema_repair::diagnostic::structured_schema_sha256,
};

use super::contracts::CompatibleRepairCheckpointInput;

impl OrchestratorContext {
    pub(crate) async fn load_compatible_full_repair_output<T, V>(
        &self,
        workflow_run_id: i64,
        base_call_key: &str,
        raw_response: &str,
        validate_output: &V,
    ) -> Result<Option<T>, AppError>
    where
        T: DeserializeOwned + JsonSchema,
        V: Fn(&T) -> Result<(), AppError>,
    {
        let schema = serde_json::to_value(schemars::schema_for!(T))?;
        let target_schema_sha256 = structured_schema_sha256(&schema)?;
        let parse_output = |value: &Value| {
            let output: T = serde_json::from_value(value.clone())?;
            validate_output(&output)?;
            Ok(output)
        };
        let value = self
            .load_compatible_structured_repair_value(CompatibleRepairCheckpointInput {
                workflow_run_id,
                base_call_key,
                call_key: &format!("{base_call_key}:schema_repair"),
                suffix: "schema_repair",
                base_response_sha256: &a4n_core::source_spans::source_text_hash(raw_response),
                target_schema_sha256: &target_schema_sha256,
                require_target_validation: true,
                parse_output: &parse_output,
            })
            .await?;
        value.map(|value| parse_output(&value)).transpose()
    }
}
