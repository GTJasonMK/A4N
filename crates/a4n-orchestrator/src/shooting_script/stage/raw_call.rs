use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{ChatJsonStreamInput, StructuredLlmProgress},
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
        fingerprint_json: &Value,
        request: ChatRequest,
        label: &str,
        structured_progress: Option<StructuredLlmProgress>,
        progress_total: u32,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<super::super::contracts::ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<RawCallResult, AppError> {
        self.mark_workflow_current_step(workflow_run_id, call_key)
            .await?;
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, call_key, fingerprint_json)
            .await?
        {
            return Ok(RawCallResult {
                response_content: checkpoint.response_content,
                restored_parsed_output: Some(checkpoint.parsed_output),
                usage: checkpoint.usage,
                finish_reason: checkpoint.finish_reason,
            });
        }

        let truncated_message = format!("{label}输出达到 max_tokens 上限，JSON 未完整闭合");
        let fixed_progress = structured_progress
            .map(|value| value.span.end())
            .unwrap_or(0);
        // 活体信号与字段级里程碑取自同一个 token 流，二者互补而非替代。
        let mut streamed = String::new();
        let mut stream_sink = stream_reporter.map(|reporter| {
            move |delta: &str| {
                streamed.push_str(delta);
                reporter(&streamed);
            }
        });
        let result = self
            .chat_json_stream_result_with_sink(
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
                stream_sink
                    .as_mut()
                    .map(|sink| sink as &mut (dyn FnMut(&str) + Send)),
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
