use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde::{Serialize, de::DeserializeOwned};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{
        StructuredLlmProgress, ensure_checkpoint_parsed_output_matches, parse_strict_json_response,
    },
    workflow::{llm_call_checkpoint_fingerprint_sha256, workflow_checkpoint_contract_app_error},
};

use super::contracts::{
    ShootingScriptProgressReporter, ShootingScriptStageResult, ShootingScriptStreamReporter,
};

mod persistence;
mod raw_call;

impl OrchestratorContext {
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
        T: Serialize + DeserializeOwned,
    {
        self.ensure_shooting_script_running(workflow_run_id).await?;
        let fingerprint_json = a4n_llm_trace::context_aware_chat_request_trace_json(&request)?;
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
                request,
                label,
                structured_progress,
                progress_total,
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
        let restored = raw.restored_parsed_output.is_some();
        let output =
            parse_strict_json_response::<T>(&raw.response_content, label).and_then(|output| {
                validate_output(&output)?;
                if let Some(saved) = raw.restored_parsed_output.as_ref() {
                    ensure_checkpoint_parsed_output_matches(
                        saved,
                        &serde_json::to_value(&output)?,
                        &format!("分镜脚本 {step_key}"),
                    )?;
                }
                Ok(output)
            });
        let output = if restored {
            self.validate_llm_call_output(workflow_run_id, step_key, &fingerprint_json, output)
                .await?
        } else {
            output?
        };
        if !restored {
            self.persist_parsed_shooting_call(
                workflow_run_id,
                step_key,
                step_order,
                fingerprint_json,
                &raw.response_content,
                &output,
                raw.usage,
                raw.finish_reason,
            )
            .await?;
        }
        Ok(ShootingScriptStageResult { output, restored })
    }
}
