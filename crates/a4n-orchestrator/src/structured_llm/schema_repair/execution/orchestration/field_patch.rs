use std::collections::HashMap;

use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    structured_llm::schema_repair::{
        contract::{SchemaFieldPatchV1, StructuredRepairMode},
        diagnostic::diagnose_structured_json_with_schema,
        execution::{contracts::StructuredValueRepairInput, rules::report_progress},
        patch::apply_schema_field_patch,
        request::build_schema_field_patch_request,
    },
};

use super::super::{contracts::StructuredOutputRepairResult, rules::is_transport_or_cancel_error};
use super::contracts::{FieldPatchOutcome, StructuredRepairState};

pub(super) async fn attempt_schema_field_patch<T, P>(
    context: &OrchestratorContext,
    input: &StructuredValueRepairInput<'_, P>,
    mut state: StructuredRepairState,
    target_schema_sha256: &str,
) -> Result<FieldPatchOutcome<T>, a4n_core::error::AppError>
where
    P: Fn(&Value) -> Result<T, a4n_core::error::AppError>,
{
    let Some(candidate) = state.candidate.clone() else {
        return Ok(FieldPatchOutcome::Continue(state));
    };
    let report = diagnose_structured_json_with_schema(&candidate, input.schema.clone())?;
    state.schema_sha256 = Some(report.schema_sha256.clone());
    if !report.field_patch_allowed {
        return Ok(FieldPatchOutcome::Continue(state));
    }

    report_progress(
        input.progress,
        &format!(
            "{}检测到 {} 个结构错误，正在生成字段补丁",
            input.label,
            report.diagnostics.len()
        ),
    )?;
    let project_id = input
        .request
        .trace_context
        .as_ref()
        .and_then(|trace| trace.project_id);
    let repair_prompt = context.prompt_render_template(
        "structured_json_field_repair",
        &HashMap::new(),
        project_id,
    )?;
    let patch_request =
        build_schema_field_patch_request(input.request.clone(), &repair_prompt, &report)?;
    match context
        .run_structured_repair_call(
            input,
            "schema_patch",
            patch_request,
            target_schema_sha256,
            Some(json!({
                "target_schema_sha256": report.schema_sha256,
                "diagnostics": report.diagnostics,
            })),
        )
        .await
    {
        Ok(call) => match serde_json::from_value::<SchemaFieldPatchV1>(call.value) {
            Ok(patch) => match apply_schema_field_patch(&candidate, &report, &patch) {
                Ok(patched) => match (input.parse_output)(&patched) {
                    Ok(output) => {
                        report_progress(
                            input.progress,
                            &format!("{}字段补丁已通过完整校验", input.label),
                        )?;
                        return Ok(FieldPatchOutcome::Complete(StructuredOutputRepairResult {
                            output,
                            final_value: patched,
                            mode: StructuredRepairMode::FieldPatch,
                            restored: call.restored,
                            schema_sha256: state.schema_sha256,
                            field_patch_error: None,
                        }));
                    }
                    Err(error) => {
                        state.field_patch_error = Some(error.to_string());
                        state.candidate = Some(patched);
                    }
                },
                Err(error) => state.field_patch_error = Some(error.to_string()),
            },
            Err(error) => state.field_patch_error = Some(error.to_string()),
        },
        Err(error) if is_transport_or_cancel_error(&error) => return Err(error),
        Err(error) => state.field_patch_error = Some(error.to_string()),
    }
    Ok(FieldPatchOutcome::Continue(state))
}
