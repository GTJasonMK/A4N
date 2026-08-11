use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{
        StructuredLlmProgress, StructuredOutputRepairInput, StructuredRepairCheckpointContext,
        parse_strict_json_response,
    },
    workflow::{llm_call_checkpoint_fingerprint_sha256, workflow_checkpoint_contract_app_error},
};

use super::contracts::{
    ShootingScriptProgressReporter, ShootingScriptStageResult, ShootingScriptStreamReporter,
};

mod persistence;
mod raw_call;

impl OrchestratorContext {
    pub(super) async fn restore_completed_shooting_script_stage<T>(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        request: &ChatRequest,
        validate_output: impl Fn(&T) -> Result<(), AppError>,
    ) -> Result<Option<T>, AppError>
    where
        T: DeserializeOwned,
    {
        let fingerprint_json = a4n_llm::trace::context_aware_chat_request_trace_json(request)?;
        let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, step_key, &fingerprint_json)
            .await?
        else {
            return Ok(None);
        };
        let checkpoint_json =
            crate::workflow::required_llm_call_checkpoint_json(checkpoint, step_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let value = checkpoint_json
            .get("parsed_output")
            .filter(|value| !value.is_null())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("{step_key}.parsed_output 必须是结构化输出"))
            })?;
        let output = serde_json::from_value(value.clone())?;
        validate_output(&output)?;
        Ok(Some(output))
    }

    pub(super) async fn run_shooting_script_stage<T>(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        mut request: ChatRequest,
        label: &str,
        structured_progress: StructuredLlmProgress,
        progress_total: u32,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        validate_output: impl Fn(&T) -> Result<(), AppError>,
    ) -> Result<ShootingScriptStageResult<T>, AppError>
    where
        T: Serialize + DeserializeOwned + JsonSchema + PartialEq,
    {
        self.ensure_shooting_script_running(workflow_run_id).await?;
        let fingerprint_json = a4n_llm::trace::context_aware_chat_request_trace_json(&request)?;
        let fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace) = request.trace_context.as_mut() {
            trace.input_fingerprint_sha256 = Some(fingerprint_sha256);
        }
        let raw = self
            .run_or_restore_raw_shooting_call(
                workflow_run_id,
                step_key,
                &fingerprint_json,
                request.clone(),
                label,
                Some(structured_progress),
                progress_total,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
        let parsed = if let Some(saved) = raw.restored_parsed_output.as_ref() {
            let output: T = serde_json::from_value(saved.clone())?;
            validate_output(&output).map(|()| output)
        } else {
            parse_strict_json_response::<T>(&raw.response_content, label)
                .and_then(|output| validate_output(&output).map(|()| output))
        };

        match parsed {
            Ok(output) => {
                if raw.restored_parsed_output.is_some() {
                    return Ok(ShootingScriptStageResult {
                        output,
                        restored: true,
                    });
                }
                self.persist_parsed_shooting_call(
                    workflow_run_id,
                    step_key,
                    step_order,
                    fingerprint_json,
                    &raw.response_content,
                    &output,
                    raw.usage,
                    raw.finish_reason,
                    None,
                )
                .await?;
                Ok(ShootingScriptStageResult {
                    output,
                    restored: false,
                })
            }
            Err(_initial_error) => {
                let repair_progress = structured_progress.span.end();
                let report_repair_progress =
                    |message: &str| progress(repair_progress, progress_total, message);
                let repaired = self
                    .parse_validate_or_repair_structured_output::<T, _>(
                        StructuredOutputRepairInput {
                            request,
                            raw_response: &raw.response_content,
                            label,
                            checkpoint: Some(StructuredRepairCheckpointContext {
                                workflow_run_id,
                                base_call_key: step_key.to_string(),
                                call_order: step_order,
                            }),
                            cancel_token,
                            progress: Some(&report_repair_progress),
                            validate_output: &validate_output,
                        },
                    )
                    .await?;
                self.persist_parsed_shooting_call(
                    workflow_run_id,
                    step_key,
                    step_order,
                    fingerprint_json,
                    &raw.response_content,
                    &repaired.output,
                    raw.usage,
                    raw.finish_reason,
                    Some(json!({
                        "structured_repair": repaired.artifact_summary_with([
                            ("status".to_string(), json!("completed")),
                            (
                                "base_response_sha256".to_string(),
                                json!(a4n_core::source_spans::source_text_hash(
                                    &raw.response_content
                                )),
                            ),
                        ]),
                    })),
                )
                .await?;
                Ok(ShootingScriptStageResult {
                    output: repaired.output,
                    restored: false,
                })
            }
        }
    }
}
