use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{ChatJsonStreamInput, StructuredLlmProgress},
    workflow::{required_llm_call_checkpoint_json, workflow_checkpoint_contract_app_error},
};

use super::super::contracts::ShootingScriptProgressReporter;

pub(super) struct RawCallResult {
    pub(super) response_content: String,
    pub(super) restored_parsed_output: Option<Value>,
    pub(super) usage: Option<Value>,
    pub(super) finish_reason: Option<String>,
}

impl OrchestratorContext {
    pub(super) async fn run_or_restore_raw_shooting_call(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        call_order: i32,
        fingerprint_json: &Value,
        request: ChatRequest,
        label: &str,
        structured_progress: Option<StructuredLlmProgress>,
        progress_total: u32,
        progress: ShootingScriptProgressReporter<'_>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<RawCallResult, AppError> {
        self.mark_workflow_current_step(workflow_run_id, call_key)
            .await?;
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint_with_repair_state(
                workflow_run_id,
                call_key,
                fingerprint_json,
            )
            .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
            let response_content = checkpoint_json["response_content"]
                .as_str()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!("{call_key}.response_content 必须是字符串"))
                })?
                .to_string();
            let restored_parsed_output = checkpoint_json
                .get("parsed_output")
                .filter(|value| !value.is_null())
                .cloned();
            let usage = checkpoint_json
                .get("usage")
                .filter(|value| !value.is_null())
                .cloned();
            let finish_reason = checkpoint_json
                .get("finish_reason")
                .and_then(Value::as_str)
                .map(str::to_string);
            return Ok(RawCallResult {
                response_content,
                restored_parsed_output,
                usage,
                finish_reason,
            });
        }

        let truncated_message = format!("{label}输出达到 max_tokens 上限，JSON 未完整闭合");
        let fixed_progress = structured_progress
            .map(|value| value.span.end())
            .unwrap_or(0);
        let result = self
            .chat_json_stream_result_with_progress(
                ChatJsonStreamInput {
                    request,
                    prompt_name: call_key,
                    truncated_message: &truncated_message,
                    structured_progress,
                    cancel_token,
                    cancelled_message: "拍摄剧本生成已停止",
                },
                |current, message| {
                    progress(
                        if structured_progress.is_some() {
                            current
                        } else {
                            fixed_progress
                        },
                        progress_total,
                        message,
                    )
                    .map_err(|error| error.to_string())
                },
            )
            .await
            .map_err(|error| {
                if cancel_token.is_some_and(CancellationToken::is_cancelled) {
                    AppError::Cancelled
                } else {
                    AppError::LlmError(error)
                }
            })?;
        if let Some(usage) = result.usage.as_ref() {
            let hit = usage.prompt_cache_hit_tokens.unwrap_or(0);
            let miss = usage.prompt_cache_miss_tokens.unwrap_or(0);
            let ratio = (hit + miss > 0).then(|| hit as f64 / (hit + miss) as f64);
            tracing::info!(
                target: "a4n::shooting_script",
                call_key,
                prompt_tokens = usage.prompt_tokens,
                completion_tokens = usage.completion_tokens,
                prompt_cache_hit_tokens = hit,
                prompt_cache_miss_tokens = miss,
                prompt_cache_hit_ratio = ratio,
                "拍摄剧本 LLM token usage"
            );
        }
        self.persist_raw_shooting_call(
            workflow_run_id,
            call_key,
            call_order,
            fingerprint_json.clone(),
            &result,
        )
        .await?;
        Ok(RawCallResult {
            response_content: result.response_content,
            restored_parsed_output: None,
            usage: result
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
            finish_reason: result.finish_reason,
        })
    }

    pub(in crate::shooting_script) async fn load_completed_llm_call_checkpoint_with_repair_state(
        &self,
        workflow_run_id: i64,
        call_key: &str,
        fingerprint_json: &Value,
    ) -> Result<Option<a4n_core::models::generation::workflow_checkpoint::Model>, AppError> {
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, fingerprint_json)
            .await?
        {
            return Ok(Some(checkpoint));
        }
        let checkpoints = self.workflow_checkpoint_models(workflow_run_id).await?;
        Ok(checkpoints.into_iter().find(|checkpoint| {
            checkpoint.step_key == call_key
                && checkpoint.status == crate::workflow::WORKFLOW_STATUS_COMPLETED
                && checkpoint.artifacts_json.as_ref().is_some_and(|artifacts| {
                    let state = artifacts.get("structured_repair_state");
                    let valid_state = state.and_then(|value| value.get("schema_version"))
                        == Some(&Value::from(1));
                    let status = state
                        .and_then(|value| value.get("status"))
                        .and_then(Value::as_str);
                    let has_parsed_output = checkpoint
                        .checkpoint_json
                        .as_ref()
                        .and_then(|value| value.get("parsed_output"))
                        .is_some_and(|value| !value.is_null());
                    let has_raw_response = checkpoint
                        .checkpoint_json
                        .as_ref()
                        .and_then(|value| value.get("response_content"))
                        .and_then(Value::as_str)
                        .is_some_and(|value| !value.trim().is_empty());
                    valid_state
                        && match status {
                            Some("completed") => has_parsed_output,
                            Some("pending_field_patch") => {
                                !call_key.ends_with(":schema_patch")
                                    && !call_key.ends_with(":schema_repair")
                                    && has_raw_response
                                    && !has_parsed_output
                            }
                            _ => false,
                        }
                })
        }))
    }

    pub(super) async fn ensure_shooting_script_running(
        &self,
        workflow_run_id: i64,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        if run.status == a4n_db::workflow::WORKFLOW_STATUS_CANCELLED {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}
