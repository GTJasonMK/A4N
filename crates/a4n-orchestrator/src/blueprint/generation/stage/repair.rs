use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::parse_blueprint_stage_json_object;
use serde_json::Value;

use crate::blueprint::OrchestratorContext;
use crate::structured_llm::{
    StructuredOutputRepairResult, StructuredRepairCheckpointContext, StructuredValueRepairInput,
    json_shape_schema_from_example,
};

use super::contracts::{BlueprintStageLlmOutput, PreparedBlueprintStage};
use super::rules::report;

pub(super) async fn repair_blueprint_stage_output(
    context: &OrchestratorContext,
    prepared: &PreparedBlueprintStage<'_>,
    call_order: i32,
    output: &BlueprintStageLlmOutput,
) -> Result<StructuredOutputRepairResult<Value>, AppError> {
    let output_schema_example: Value =
        serde_json::from_str(prepared.spec.output_schema).map_err(|error| {
            AppError::ConfigError(format!(
                "{} output_schema 不是合法 JSON：{error}",
                prepared.spec.title
            ))
        })?;
    let output_schema = json_shape_schema_from_example(&output_schema_example);
    let parse_output = |value: &Value| {
        let response = serde_json::to_string(value)?;
        let parsed = parse_blueprint_stage_json_object(
            &response,
            prepared.spec.title,
            prepared.spec.expected_top_level_keys,
        )?;
        (prepared.spec.validate)(&parsed)?;
        Ok(parsed)
    };
    let report_repair_progress =
        |message: &str| report(prepared.progress, prepared.spec.progress_current, message);
    context
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: prepared.request.clone(),
            raw_response: &output.response_content,
            label: prepared.spec.title,
            schema: output_schema,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id: prepared.workflow_run_id,
                base_call_key: prepared.call_key.clone(),
                call_order,
            }),
            cancel_token: prepared.cancel_token,
            progress: Some(&report_repair_progress),
            parse_output: &parse_output,
        })
        .await
}
