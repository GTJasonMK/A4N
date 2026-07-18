use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    structured_llm::schema_repair::{
        contract::StructuredRepairMode,
        execution::{
            contracts::{StructuredOutputRepairResult, StructuredValueRepairInput},
            rules::report_progress,
        },
        request::build_full_schema_repair_request,
    },
};

use super::contracts::StructuredRepairState;

pub(super) async fn run_full_schema_repair<T, P>(
    context: &OrchestratorContext,
    input: &StructuredValueRepairInput<'_, P>,
    state: StructuredRepairState,
    target_schema_sha256: &str,
) -> Result<StructuredOutputRepairResult<T>, a4n_core::error::AppError>
where
    P: Fn(&Value) -> Result<T, a4n_core::error::AppError>,
{
    report_progress(
        input.progress,
        &format!("{}正在执行一次完整结构纠正", input.label),
    )?;
    let invalid_response = state
        .candidate
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?
        .unwrap_or_else(|| input.raw_response.to_string());
    let full_request = build_full_schema_repair_request(
        input.request.clone(),
        &invalid_response,
        &state.initial_error.to_string(),
    )?;
    let full_call = context
        .run_structured_repair_call(
            input,
            "schema_repair",
            full_request,
            target_schema_sha256,
            Some(json!({
                "field_patch_error": &state.field_patch_error,
            })),
        )
        .await
        .map_err(|repair_error| {
            a4n_core::error::AppError::InvalidArgument(format!(
                "{}首次结构校验失败：{}{}；一次完整结构纠正调用失败：{}",
                input.label,
                state.initial_error,
                state
                    .field_patch_error
                    .as_ref()
                    .map(|error| format!("；字段补丁失败：{error}"))
                    .unwrap_or_default(),
                repair_error
            ))
        })?;
    let output = (input.parse_output)(&full_call.value).map_err(|repair_error| {
        a4n_core::error::AppError::InvalidArgument(format!(
            "{}首次结构校验失败：{}{}；一次完整结构纠正后仍不合法：{}",
            input.label,
            state.initial_error,
            state
                .field_patch_error
                .as_ref()
                .map(|error| format!("；字段补丁失败：{error}"))
                .unwrap_or_default(),
            repair_error
        ))
    })?;
    Ok(StructuredOutputRepairResult {
        output,
        final_value: full_call.value,
        mode: StructuredRepairMode::FullObjectRepair,
        restored: full_call.restored,
        schema_sha256: state.schema_sha256,
        field_patch_error: state.field_patch_error,
    })
}
