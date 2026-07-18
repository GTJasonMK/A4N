use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::{
    OrchestratorContext, structured_llm::schema_repair::diagnostic::structured_schema_sha256,
};

use super::super::contracts::{
    StructuredOutputRepairInput, StructuredOutputRepairResult, StructuredValueRepairInput,
};
use super::{
    contracts::{FieldPatchOutcome, InitialRepairOutcome},
    field_patch::attempt_schema_field_patch,
    full_repair::run_full_schema_repair,
    initial::parse_initial_structured_output,
};

impl OrchestratorContext {
    pub async fn parse_validate_or_repair_structured_output<T, V>(
        &self,
        input: StructuredOutputRepairInput<'_, V>,
    ) -> Result<StructuredOutputRepairResult<T>, a4n_core::error::AppError>
    where
        T: Serialize + DeserializeOwned + JsonSchema,
        V: Fn(&T) -> Result<(), a4n_core::error::AppError>,
    {
        let schema = serde_json::to_value(schemars::schema_for!(T))?;
        let parse_output = |value: &Value| {
            let output: T = serde_json::from_value(value.clone())?;
            (input.validate_output)(&output)?;
            Ok(output)
        };
        self.parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: input.request,
            raw_response: input.raw_response,
            label: input.label,
            schema,
            checkpoint: input.checkpoint,
            cancel_token: input.cancel_token,
            progress: input.progress,
            parse_output: &parse_output,
        })
        .await
    }

    pub async fn parse_validate_or_repair_structured_value<T, P>(
        &self,
        input: StructuredValueRepairInput<'_, P>,
    ) -> Result<StructuredOutputRepairResult<T>, a4n_core::error::AppError>
    where
        P: Fn(&Value) -> Result<T, a4n_core::error::AppError>,
    {
        let target_schema_sha256 = structured_schema_sha256(&input.schema)?;
        let state = match parse_initial_structured_output(&input) {
            InitialRepairOutcome::Complete(result) => return Ok(result),
            InitialRepairOutcome::Repair(state) => state,
        };
        let state =
            match attempt_schema_field_patch(self, &input, state, &target_schema_sha256).await? {
                FieldPatchOutcome::Complete(result) => return Ok(result),
                FieldPatchOutcome::Continue(state) => state,
            };
        run_full_schema_repair(self, &input, state, &target_schema_sha256).await
    }
}
