use std::{sync::Arc, time::Duration};

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{
    ChatMessage, ChatRequest, ChatResponse, LlmProvider, UsageInfo, is_truncated_finish_reason,
};
use a4n_progress::ProgressPhaseKind;
use a4n_service_protocol::{
    GenerationError, GenerationErrorCode, GenerationFinishReason, GenerationUsage,
    JS_SAFE_INTEGER_U64, MAX_TEXT_DELTA_BYTES, ProgressEvent, UsageEvent,
};
use async_trait::async_trait;
use futures::{StreamExt, future::pending};
use sha2::{Digest, Sha256};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

use super::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::{
        GenerationEventPublisher, GenerationSessionError, GenerationTaskSession,
        SessionGenerationEventPublisher,
    },
    generation_finish_reason_from_text, generation_finish_reason_text, generation_progress,
};
use crate::OrchestratorContext;

const BATCH_DELAY: Duration = Duration::from_millis(50);
const BATCH_BYTES: usize = MAX_TEXT_DELTA_BYTES;
pub(crate) const REASONING_STREAM_ID: &str = "reasoning";

pub(crate) struct LlmGenerationCheckpoint<'a> {
    pub(crate) writer: &'a GenerationCheckpointWriter,
    pub(crate) session: &'a Arc<GenerationTaskSession>,
    pub(crate) metadata: GenerationCheckpointMetadata,
}

pub(crate) struct LlmGenerationRequest<'a> {
    pub(crate) request: ChatRequest,
    pub(crate) stream_id: String,
    pub(crate) cancellation: CancellationToken,
    pub(crate) checkpoint: Option<LlmGenerationCheckpoint<'a>>,
    pub(crate) usage_base: Option<GenerationUsage>,
}

#[derive(Clone)]
pub(crate) struct TaskLlmStreamContext {
    pub(crate) publisher: SessionGenerationEventPublisher,
    pub(crate) session: Arc<GenerationTaskSession>,
    pub(crate) checkpoint_writer: GenerationCheckpointWriter,
    pub(crate) cancellation: CancellationToken,
}

#[derive(Debug, Clone)]
pub(crate) struct LlmGenerationOutput {
    pub(crate) content: String,
    pub(crate) usage: Option<GenerationUsage>,
    pub(crate) finish_reason: GenerationFinishReason,
    pub(crate) content_sha256: String,
}

/// 工作流领域在尚未接入 generation session 时使用的 publisher。
/// 仍走统一 stream driver（超时/截断/finish_reason）；有 session 的任务必须用
/// `SessionGenerationEventPublisher`。
struct WorkflowStreamPublisher<'a> {
    stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
}

#[async_trait]
impl GenerationEventPublisher for WorkflowStreamPublisher<'_> {
    async fn progress(&self, _event: ProgressEvent) -> Result<(), GenerationSessionError> {
        Ok(())
    }

    async fn text_delta(&self, stream_id: &str, delta: &str) -> Result<(), GenerationSessionError> {
        if stream_id == "main"
            && let Some(reporter) = self.stream_reporter
        {
            reporter(delta);
        }
        Ok(())
    }

    async fn usage(&self, _event: UsageEvent) -> Result<(), GenerationSessionError> {
        Ok(())
    }
}

/// 生产完整响应替代入口：真实 `chat_stream` + 统一超时/截断规则，返回 ChatResponse。
pub(crate) async fn execute_llm_chat_response(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancellation: CancellationToken,
) -> Result<ChatResponse, AppError> {
    execute_llm_chat_response_with_sink(llm, request, cancellation, None).await
}

pub(crate) async fn execute_llm_chat_response_with_sink(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancellation: CancellationToken,
    stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
) -> Result<ChatResponse, AppError> {
    let publisher = WorkflowStreamPublisher { stream_reporter };
    let output = execute_llm_generation(
        llm,
        &publisher,
        LlmGenerationRequest {
            request,
            stream_id: "main".to_string(),
            cancellation,
            checkpoint: None,
            usage_base: None,
        },
    )
    .await
    .map_err(map_generation_error_to_app)?;
    Ok(ChatResponse {
        message: ChatMessage::assistant(output.content),
        usage: match output.usage.as_ref() {
            Some(usage) => {
                Some(provider_usage(usage).map_err(|error| AppError::LlmError(error.message))?)
            }
            None => None,
        },
        finish_reason: Some(generation_finish_reason_text(output.finish_reason)),
        reasoning_content: None,
    })
}

pub(crate) async fn execute_task_llm_chat_response<T>(
    context: &OrchestratorContext,
    stream: &TaskLlmStreamContext,
    request: ChatRequest,
    call_key: &str,
    step_key: &str,
    validate: impl FnMut(&str) -> Result<T, AppError>,
) -> Result<(ChatResponse, T), AppError> {
    if stream.cancellation.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let (output, parsed) = execute_validated_llm_generation(
        context.llm_backend(),
        &stream.publisher,
        LlmGenerationRequest {
            request,
            stream_id: "raw_json".to_string(),
            cancellation: stream.cancellation.clone(),
            checkpoint: Some(LlmGenerationCheckpoint {
                writer: &stream.checkpoint_writer,
                session: &stream.session,
                metadata: GenerationCheckpointMetadata {
                    step_key: Some(step_key.to_string()),
                    call_key: Some(call_key.to_string()),
                    provider_model: Some(context.llm_model_name()),
                    ..Default::default()
                },
            }),
            usage_base: stream.session.snapshot().usage,
        },
        call_key,
        validate,
    )
    .await
    .map_err(map_generation_error_to_app)?;
    Ok((
        ChatResponse {
            message: ChatMessage::assistant(output.content),
            usage: match output.usage.as_ref() {
                Some(usage) => {
                    Some(provider_usage(usage).map_err(|error| AppError::LlmError(error.message))?)
                }
                None => None,
            },
            finish_reason: Some(generation_finish_reason_text(output.finish_reason)),
            reasoning_content: None,
        },
        parsed,
    ))
}

fn map_generation_error_to_app(error: GenerationError) -> AppError {
    if error.code.as_str() == "generation_interrupted" {
        return AppError::Cancelled;
    }
    if error.code.as_str() == "llm_output_truncated" {
        return AppError::InvalidArgument(error.message);
    }
    AppError::LlmError(error.message)
}

pub(crate) async fn execute_llm_generation(
    llm: &dyn LlmProvider,
    publisher: &dyn GenerationEventPublisher,
    input: LlmGenerationRequest<'_>,
) -> Result<LlmGenerationOutput, GenerationError> {
    let LlmGenerationRequest {
        request,
        stream_id,
        cancellation,
        checkpoint,
        usage_base,
    } = input;
    publish_progress(
        publisher,
        "provider_connect",
        ProgressPhaseKind::ModelConnect,
        "正在连接模型",
    )
    .await?;
    let mut stream = a4n_llm_provider_contract::provider::open_chat_stream(
        llm.chat_stream(request),
        cancellation.clone(),
    )
    .await
    .map_err(|error| {
        if matches!(error, AppError::Cancelled) {
            return cancelled_error();
        }
        tracing::error!(stage_code = "provider_connect", error = %error, "模型流建立失败");
        let retryable = matches!(error, AppError::Timeout(_) | AppError::LlmError(_));
        public_error(
            "provider_job_failed",
            &format!(
                "模型调用准备或建流失败：{}",
                a4n_config::app_error_plain_message(error)
            ),
            retryable,
            "provider_connect",
        )
    })?;
    publish_progress(
        publisher,
        "provider_wait",
        ProgressPhaseKind::ModelWait,
        "模型已连接，正在等待输出",
    )
    .await?;
    checkpoint_if_due(checkpoint.as_ref()).await?;

    let mut flush_deadline = None;
    let mut saw_content = false;
    let mut content = String::new();
    let mut pending_text = String::new();
    let mut pending_reasoning = String::new();
    let mut usage = None;
    let mut finish_reason = None;

    loop {
        flush_full_batches(publisher, &stream_id, &mut pending_text).await?;
        flush_full_batches(publisher, REASONING_STREAM_ID, &mut pending_reasoning).await?;
        checkpoint_if_due(checkpoint.as_ref()).await?;
        if pending_text.is_empty() && pending_reasoning.is_empty() {
            flush_deadline = None;
        }

        enum Next<T> {
            Item(T),
            Flush,
            Cancelled,
        }
        let next = tokio::select! {
            _ = cancellation.cancelled() => Next::Cancelled,
            _ = wait_until(flush_deadline) => Next::Flush,
            item = stream.next() => Next::Item(item),
        };

        match next {
            Next::Flush => {
                flush_text(publisher, &stream_id, &mut pending_text).await?;
                flush_text(publisher, REASONING_STREAM_ID, &mut pending_reasoning).await?;
                flush_deadline = None;
            }
            Next::Cancelled => {
                flush_text(publisher, &stream_id, &mut pending_text).await?;
                flush_text(publisher, REASONING_STREAM_ID, &mut pending_reasoning).await?;
                return Err(cancelled_error());
            }
            Next::Item(None) => {
                flush_text(publisher, &stream_id, &mut pending_text).await?;
                flush_text(publisher, REASONING_STREAM_ID, &mut pending_reasoning).await?;
                if let Some(finish_reason) = finish_reason {
                    let content_sha256 = format!("{:x}", Sha256::digest(content.as_bytes()));
                    return Ok(LlmGenerationOutput {
                        content,
                        usage,
                        finish_reason,
                        content_sha256,
                    });
                }
                return Err(public_error(
                    "llm_stream_missing_finish_reason",
                    "模型流结束但缺少 finish_reason",
                    true,
                    "provider_stream",
                ));
            }
            Next::Item(Some(Err(error))) => {
                flush_text(publisher, &stream_id, &mut pending_text).await?;
                flush_text(publisher, REASONING_STREAM_ID, &mut pending_reasoning).await?;
                if matches!(error, AppError::Cancelled) {
                    return Err(cancelled_error());
                }
                tracing::error!(
                    stage_code = "provider_stream",
                    error = %error,
                    "模型流读取失败"
                );
                let message = format!(
                    "模型流读取失败：{}",
                    a4n_config::app_error_plain_message(error)
                );
                return Err(public_error(
                    "provider_job_failed",
                    &message,
                    true,
                    "provider_stream",
                ));
            }
            Next::Item(Some(Ok(chunk))) => {
                if let Some(delta) = chunk.content.as_deref().filter(|delta| !delta.is_empty()) {
                    if !saw_content {
                        saw_content = true;
                        publish_progress(
                            publisher,
                            "provider_stream",
                            ProgressPhaseKind::ModelStream,
                            "模型已开始输出",
                        )
                        .await?;
                    }
                    content.push_str(delta);
                    pending_text.push_str(delta);
                    flush_deadline.get_or_insert(Instant::now() + BATCH_DELAY);
                }
                if let Some(delta) = chunk
                    .reasoning_content
                    .as_deref()
                    .filter(|delta| !delta.is_empty())
                {
                    pending_reasoning.push_str(delta);
                    flush_deadline.get_or_insert(Instant::now() + BATCH_DELAY);
                }
                if let Some(chunk_usage) = chunk.usage {
                    let cumulative = generation_usage(chunk_usage)?;
                    publish_generation_usage(publisher, &cumulative, usage_base.as_ref()).await?;
                    usage = Some(cumulative);
                }
                if let Some(reason) = chunk.finish_reason.as_deref() {
                    flush_text(publisher, &stream_id, &mut pending_text).await?;
                    flush_text(publisher, REASONING_STREAM_ID, &mut pending_reasoning).await?;
                    finish_reason = Some(map_finish_reason(reason)?);
                }
            }
        }
    }
}

struct ValidatedOutputPublisher<'a> {
    inner: &'a dyn GenerationEventPublisher,
    adopted_stream: &'a str,
}

#[async_trait]
impl GenerationEventPublisher for ValidatedOutputPublisher<'_> {
    async fn progress(&self, event: ProgressEvent) -> Result<(), GenerationSessionError> {
        self.inner.progress(event).await
    }

    async fn text_delta(&self, stream_id: &str, delta: &str) -> Result<(), GenerationSessionError> {
        if stream_id != self.adopted_stream {
            return self.inner.text_delta(stream_id, delta).await;
        }
        self.inner
            .progress(generation_progress(
                "provider_read",
                ProgressPhaseKind::ModelStream,
                "模型正在输出，完整结果校验通过后采用",
            ))
            .await
    }

    async fn usage(&self, _event: UsageEvent) -> Result<(), GenerationSessionError> {
        // Provider calls keep their own billing records. Publish an adopted
        // logical-call meter only after all attempt meters can be combined.
        Ok(())
    }
}

pub(crate) async fn execute_validated_llm_generation<T>(
    llm: &dyn LlmProvider,
    publisher: &dyn GenerationEventPublisher,
    mut input: LlmGenerationRequest<'_>,
    context: &str,
    validate: impl FnMut(&str) -> Result<T, AppError>,
) -> Result<(LlmGenerationOutput, T), GenerationError> {
    let request = input.request.clone();
    let cancellation = input.cancellation.clone();
    let stream_id = input.stream_id.clone();
    let base = input.usage_base.take();
    let buffered = ValidatedOutputPublisher {
        inner: publisher,
        adopted_stream: &stream_id,
    };
    let initial = execute_llm_generation(llm, &buffered, input).await?;
    let initial = ChatResponse {
        message: ChatMessage::assistant(initial.content),
        usage: initial.usage.as_ref().map(provider_usage).transpose()?,
        finish_reason: Some(generation_finish_reason_text(initial.finish_reason)),
        reasoning_content: None,
    };
    let (response, parsed) = crate::structured_llm::correct_llm_response(
        &request,
        initial,
        context,
        Some(&cancellation),
        validate,
        |corrective_request| async {
            let output = execute_llm_generation(
                llm,
                &buffered,
                LlmGenerationRequest {
                    request: corrective_request,
                    stream_id: stream_id.clone(),
                    cancellation: cancellation.clone(),
                    // A rejected attempt must never overwrite a completed business checkpoint.
                    checkpoint: None,
                    usage_base: None,
                },
            )
            .await
            .map_err(map_generation_error_to_app)?;
            Ok(ChatResponse {
                message: ChatMessage::assistant(output.content),
                usage: output
                    .usage
                    .as_ref()
                    .map(provider_usage)
                    .transpose()
                    .map_err(map_generation_error_to_app)?,
                finish_reason: Some(generation_finish_reason_text(output.finish_reason)),
                reasoning_content: None,
            })
        },
    )
    .await
    .map_err(|error| {
        if matches!(error, AppError::Cancelled) {
            cancelled_error()
        } else {
            public_error(
                "generation_validation_failed",
                &error.to_string(),
                false,
                "model_output_validation",
            )
        }
    })?;
    let content = response.message.content;
    let usage = response.usage.map(generation_usage).transpose()?;
    if let Some(usage) = usage.as_ref() {
        publish_generation_usage(publisher, usage, base.as_ref()).await?;
    }
    // Publishing rejected attempts would break the adopted stream's hash.
    // Progress/reasoning and provider traces retain diagnostics while collecting.
    for delta in crate::generation::checkpoint::split_text_delta(&content) {
        publisher
            .text_delta(&stream_id, &delta)
            .await
            .map_err(session_error)?;
    }
    let finish_reason = response
        .finish_reason
        .as_deref()
        .and_then(generation_finish_reason_from_text)
        .ok_or_else(|| {
            public_error(
                "generation_validation_failed",
                "纠正响应缺少有效结束原因",
                false,
                "model_output_validation",
            )
        })?;
    Ok((
        LlmGenerationOutput {
            content_sha256: format!("{:x}", Sha256::digest(content.as_bytes())),
            content,
            usage,
            finish_reason,
        },
        parsed,
    ))
}

pub(crate) async fn publish_generation_usage(
    publisher: &dyn GenerationEventPublisher,
    usage: &GenerationUsage,
    usage_base: Option<&GenerationUsage>,
) -> Result<(), GenerationError> {
    publisher
        .usage(UsageEvent {
            usage: accumulated_generation_usage(usage_base, usage)?,
            cumulative: true,
        })
        .await
        .map_err(session_error)
}

fn accumulated_generation_usage(
    base: Option<&GenerationUsage>,
    current: &GenerationUsage,
) -> Result<GenerationUsage, GenerationError> {
    let Some(base) = base else {
        return Ok(current.clone());
    };
    Ok(GenerationUsage {
        prompt_tokens: add_tokens(base.prompt_tokens, current.prompt_tokens)?,
        completion_tokens: add_tokens(base.completion_tokens, current.completion_tokens)?,
        total_tokens: add_tokens(base.total_tokens, current.total_tokens)?,
        prompt_cache_hit_tokens: add_optional_tokens(
            base.prompt_cache_hit_tokens,
            current.prompt_cache_hit_tokens,
        )?,
        prompt_cache_miss_tokens: add_optional_tokens(
            base.prompt_cache_miss_tokens,
            current.prompt_cache_miss_tokens,
        )?,
        prompt_cache_write_tokens: add_optional_tokens(
            base.prompt_cache_write_tokens,
            current.prompt_cache_write_tokens,
        )?,
    })
}

fn add_optional_tokens(
    left: Option<u64>,
    right: Option<u64>,
) -> Result<Option<u64>, GenerationError> {
    match (left, right) {
        (None, None) => Ok(None),
        (left, right) => add_tokens(left.unwrap_or(0), right.unwrap_or(0)).map(Some),
    }
}

fn add_tokens(left: u64, right: u64) -> Result<u64, GenerationError> {
    left.checked_add(right)
        .filter(|value| *value <= JS_SAFE_INTEGER_U64)
        .ok_or_else(|| {
            public_error(
                "generation_event_invalid",
                "模型累计 usage 超出协议范围",
                false,
                "provider_stream",
            )
        })
}

async fn checkpoint_if_due(
    checkpoint: Option<&LlmGenerationCheckpoint<'_>>,
) -> Result<(), GenerationError> {
    let Some(checkpoint) = checkpoint else {
        return Ok(());
    };
    checkpoint
        .writer
        .persist_if_due(checkpoint.session, checkpoint.metadata.clone(), false)
        .await
        .map(|_| ())
        .map_err(|error| {
            public_error(
                "generation_checkpoint_unavailable",
                &format!("generation checkpoint 写入失败: {error}"),
                true,
                "checkpoint",
            )
        })
}

async fn publish_progress(
    publisher: &dyn GenerationEventPublisher,
    stage_code: &str,
    phase_kind: ProgressPhaseKind,
    message: &str,
) -> Result<(), GenerationError> {
    publisher
        .progress(generation_progress(stage_code, phase_kind, message))
        .await
        .map_err(session_error)
}

async fn flush_full_batches(
    publisher: &dyn GenerationEventPublisher,
    stream_id: &str,
    pending_text: &mut String,
) -> Result<(), GenerationError> {
    while pending_text.len() >= BATCH_BYTES {
        let split = char_boundary_at_or_before(pending_text, BATCH_BYTES);
        if split == 0 {
            break;
        }
        let rest = pending_text.split_off(split);
        let batch = std::mem::replace(pending_text, rest);
        publisher
            .text_delta(stream_id, &batch)
            .await
            .map_err(session_error)?;
    }
    Ok(())
}

async fn flush_text(
    publisher: &dyn GenerationEventPublisher,
    stream_id: &str,
    pending_text: &mut String,
) -> Result<(), GenerationError> {
    flush_full_batches(publisher, stream_id, pending_text).await?;
    if !pending_text.is_empty() {
        publisher
            .text_delta(stream_id, pending_text)
            .await
            .map_err(session_error)?;
        pending_text.clear();
    }
    Ok(())
}

fn char_boundary_at_or_before(text: &str, target: usize) -> usize {
    let mut split = target.min(text.len());
    while split > 0 && !text.is_char_boundary(split) {
        split -= 1;
    }
    split
}

async fn wait_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => pending::<()>().await,
    }
}

pub(crate) fn generation_usage(usage: UsageInfo) -> Result<GenerationUsage, GenerationError> {
    Ok(GenerationUsage {
        prompt_tokens: safe_tokens(usage.prompt_tokens)?,
        completion_tokens: safe_tokens(usage.completion_tokens)?,
        total_tokens: safe_tokens(usage.total_tokens)?,
        prompt_cache_hit_tokens: usage.prompt_cache_hit_tokens.map(safe_tokens).transpose()?,
        prompt_cache_miss_tokens: usage
            .prompt_cache_miss_tokens
            .map(safe_tokens)
            .transpose()?,
        prompt_cache_write_tokens: usage
            .prompt_cache_write_tokens
            .map(safe_tokens)
            .transpose()?,
    })
}

pub(crate) fn provider_usage(usage: &GenerationUsage) -> Result<UsageInfo, GenerationError> {
    Ok(UsageInfo {
        prompt_tokens: usize::try_from(usage.prompt_tokens).map_err(|_| {
            public_error(
                "generation_event_invalid",
                "模型 usage 超出本地范围",
                false,
                "provider_stream",
            )
        })?,
        completion_tokens: usize::try_from(usage.completion_tokens).map_err(|_| {
            public_error(
                "generation_event_invalid",
                "模型 usage 超出本地范围",
                false,
                "provider_stream",
            )
        })?,
        total_tokens: usize::try_from(usage.total_tokens).map_err(|_| {
            public_error(
                "generation_event_invalid",
                "模型 usage 超出本地范围",
                false,
                "provider_stream",
            )
        })?,
        prompt_cache_hit_tokens: usage
            .prompt_cache_hit_tokens
            .map(usize::try_from)
            .transpose()
            .map_err(|_| {
                public_error(
                    "generation_event_invalid",
                    "模型 usage 超出本地范围",
                    false,
                    "provider_stream",
                )
            })?,
        prompt_cache_miss_tokens: usage
            .prompt_cache_miss_tokens
            .map(usize::try_from)
            .transpose()
            .map_err(|_| {
                public_error(
                    "generation_event_invalid",
                    "模型 usage 超出本地范围",
                    false,
                    "provider_stream",
                )
            })?,
        prompt_cache_write_tokens: usage
            .prompt_cache_write_tokens
            .map(usize::try_from)
            .transpose()
            .map_err(|_| {
                public_error(
                    "generation_event_invalid",
                    "模型 usage 超出本地范围",
                    false,
                    "provider_stream",
                )
            })?,
    })
}

fn safe_tokens(value: usize) -> Result<u64, GenerationError> {
    u64::try_from(value).map_err(|_| {
        public_error(
            "generation_event_invalid",
            "模型 usage 超出协议范围",
            false,
            "provider_stream",
        )
    })
}

fn map_finish_reason(reason: &str) -> Result<GenerationFinishReason, GenerationError> {
    if is_truncated_finish_reason(Some(reason)) {
        return Err(public_error(
            "llm_output_truncated",
            "模型输出被截断",
            true,
            "provider_stream",
        ));
    }
    super::generation_finish_reason_from_text(reason).ok_or_else(|| {
        public_error(
            "provider_job_failed",
            "模型返回了不支持的 finish_reason",
            false,
            "provider_stream",
        )
    })
}

fn session_error(error: super::events::GenerationSessionError) -> GenerationError {
    public_error(
        error.code(),
        "generation session 无法持久接收模型事件",
        true,
        "checkpoint",
    )
}

fn cancelled_error() -> GenerationError {
    public_error(
        "generation_interrupted",
        "生成任务已取消",
        true,
        "provider_stream",
    )
}

fn public_error(code: &str, message: &str, retryable: bool, stage_code: &str) -> GenerationError {
    GenerationError {
        code: GenerationErrorCode::registered(code).expect("registered generation error code"),
        message: message.to_string(),
        retryable,
        stage_code: stage_code.to_string(),
        trace_id: None,
        source_error: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{pin::Pin, sync::Mutex};

    use a4n_error::AppError;
    use a4n_llm_provider_contract::provider::{
        ChatChunk, ChatMessage, ModelCapabilities, UsageInfo,
    };
    use async_trait::async_trait;
    use futures::{Stream, stream};

    use super::*;

    struct FakeProvider(Vec<ChatChunk>);

    struct FailingProvider;

    #[async_trait]
    impl LlmProvider for FakeProvider {
        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
        {
            Ok(Box::pin(stream::iter(self.0.clone().into_iter().map(Ok))))
        }

        fn model_name(&self) -> &str {
            "fake"
        }

        fn capabilities(&self) -> ModelCapabilities {
            ModelCapabilities {
                max_context_tokens: 4096,
                max_output_tokens: 4096,
            }
        }

        fn count_tokens(&self, text: &str) -> usize {
            text.len()
        }
    }

    #[async_trait]
    impl LlmProvider for FailingProvider {
        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
        {
            Ok(Box::pin(stream::iter([Err(AppError::LlmError(
                "上游在接收 128 bytes 后提前断流".to_string(),
            ))])))
        }

        fn model_name(&self) -> &str {
            "failing-provider"
        }

        fn capabilities(&self) -> ModelCapabilities {
            ModelCapabilities {
                max_context_tokens: 8192,
                max_output_tokens: 4096,
            }
        }

        fn count_tokens(&self, text: &str) -> usize {
            text.len()
        }
    }

    #[derive(Default)]
    struct RecordingPublisher {
        text: Mutex<Vec<String>>,
        reasoning: Mutex<Vec<String>>,
        usage: Mutex<Option<GenerationUsage>>,
        streams: Mutex<std::collections::HashMap<String, String>>,
    }

    #[async_trait]
    impl GenerationEventPublisher for RecordingPublisher {
        async fn progress(
            &self,
            _event: ProgressEvent,
        ) -> Result<(), super::super::events::GenerationSessionError> {
            Ok(())
        }

        async fn text_delta(
            &self,
            stream_id: &str,
            delta: &str,
        ) -> Result<(), super::super::events::GenerationSessionError> {
            self.streams
                .lock()
                .unwrap()
                .entry(stream_id.to_string())
                .or_default()
                .push_str(delta);
            let target = if stream_id == REASONING_STREAM_ID {
                &self.reasoning
            } else {
                &self.text
            };
            target.lock().unwrap().push(delta.to_string());
            Ok(())
        }

        async fn usage(
            &self,
            event: UsageEvent,
        ) -> Result<(), super::super::events::GenerationSessionError> {
            *self.usage.lock().unwrap() = Some(event.usage);
            Ok(())
        }
    }

    #[tokio::test]
    async fn validated_generation_keeps_rejected_attempts_out_of_the_adopted_stream() {
        let provider =
            a4n_test_support::llm::SequenceLlmProvider::new(vec![r#"{"id":"1"}"#, r#"{"id":1}"#]);
        let publisher = RecordingPublisher::default();
        let (output, id) = execute_validated_llm_generation(
            &provider,
            &publisher,
            LlmGenerationRequest {
                request: ChatRequest::system_user("JSON integer id", "task", None, Some(100)),
                stream_id: "raw_json".into(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: None,
            },
            "test",
            |raw| {
                let value: serde_json::Value = serde_json::from_str(raw)?;
                value["id"]
                    .as_i64()
                    .ok_or_else(|| AppError::LlmError("id must be integer".into()))
            },
        )
        .await
        .unwrap();
        assert_eq!(id, 1);
        let streams = publisher.streams.lock().unwrap();
        assert_eq!(
            streams.len(),
            1,
            "rejected content must not become a public text stream"
        );
        assert_eq!(streams["raw_json"], output.content);
        assert_eq!(
            output.content_sha256,
            format!("{:x}", Sha256::digest(streams["raw_json"].as_bytes()))
        );
    }

    #[tokio::test]
    async fn unknown_initial_usage_does_not_reset_previous_task_usage_during_correction() {
        struct UsageOnCorrection(bool);
        #[async_trait]
        impl LlmProvider for UsageOnCorrection {
            async fn chat_stream(
                &self,
                request: ChatRequest,
            ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>
            {
                let corrected = request
                    .messages
                    .iter()
                    .any(|message| message.content.contains("model_output_correction"));
                FakeProvider(vec![ChatChunk {
                    content: Some(if corrected { r#"{"id":1}"# } else { "invalid" }.to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                    finish_reason: Some("stop".into()),
                    index: 0,
                    usage: (corrected || self.0).then_some(UsageInfo {
                        prompt_tokens: 3,
                        completion_tokens: 4,
                        total_tokens: 7,
                        prompt_cache_hit_tokens: None,
                        prompt_cache_miss_tokens: None,
                        prompt_cache_write_tokens: None,
                    }),
                }])
                .chat_stream(request)
                .await
            }
            fn model_name(&self) -> &str {
                "usage-on-correction"
            }
            fn capabilities(&self) -> ModelCapabilities {
                FakeProvider(Vec::new()).capabilities()
            }
        }
        let base = GenerationUsage {
            prompt_tokens: 60,
            completion_tokens: 40,
            total_tokens: 100,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
            prompt_cache_write_tokens: None,
        };
        let publisher = RecordingPublisher::default();
        *publisher.usage.lock().unwrap() = Some(base.clone());
        let (output, _) = execute_validated_llm_generation(
            &UsageOnCorrection(false),
            &publisher,
            LlmGenerationRequest {
                request: ChatRequest::system_user("JSON", "task", None, Some(100)),
                stream_id: "raw_json".into(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: Some(base.clone()),
            },
            "test",
            |raw| serde_json::from_str::<serde_json::Value>(raw).map_err(AppError::Serialization),
        )
        .await
        .unwrap();
        assert!(
            output.usage.is_none(),
            "one missing call meter must not become a supposedly complete total"
        );
        assert_eq!(
            publisher
                .usage
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .total_tokens,
            100
        );
        let publisher = RecordingPublisher::default();
        *publisher.usage.lock().unwrap() = Some(base.clone());
        let (output, _) = execute_validated_llm_generation(
            &UsageOnCorrection(true),
            &publisher,
            LlmGenerationRequest {
                request: ChatRequest::system_user("JSON", "task", None, Some(100)),
                stream_id: "raw_json".into(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: Some(base),
            },
            "test",
            |raw| serde_json::from_str::<serde_json::Value>(raw).map_err(AppError::Serialization),
        )
        .await
        .unwrap();
        assert_eq!(output.usage.unwrap().total_tokens, 14);
        assert_eq!(
            publisher
                .usage
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .total_tokens,
            114
        );
    }

    #[tokio::test]
    async fn batches_unicode_without_changing_final_content() {
        let text = "镜".repeat(BATCH_BYTES / "镜".len() + 2);
        let provider = FakeProvider(vec![
            ChatChunk {
                content: Some(text.clone()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: None,
                index: 0,
                usage: None,
            },
            ChatChunk {
                content: None,
                reasoning_content: None,
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
                index: 1,
                usage: Some(UsageInfo {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                    prompt_cache_hit_tokens: None,
                    prompt_cache_miss_tokens: None,
                    prompt_cache_write_tokens: None,
                }),
            },
        ]);
        let publisher = RecordingPublisher::default();
        let output = execute_llm_generation(
            &provider,
            &publisher,
            LlmGenerationRequest {
                request: ChatRequest::new(vec![ChatMessage::user("写作")], None, None),
                stream_id: "main".to_string(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.content, text);
        assert_eq!(publisher.text.lock().unwrap().concat(), text);
        assert!(publisher.text.lock().unwrap().len() > 1);
        assert!(
            publisher
                .text
                .lock()
                .unwrap()
                .iter()
                .all(|part| part.len() <= BATCH_BYTES)
        );
        assert_eq!(output.finish_reason, GenerationFinishReason::Stop);
        assert_eq!(output.usage.unwrap().total_tokens, 30);
    }

    #[tokio::test]
    async fn publishes_reasoning_without_polluting_response_content() {
        let provider = FakeProvider(vec![
            ChatChunk {
                content: None,
                reasoning_content: Some("先分析".to_string()),
                tool_calls: None,
                finish_reason: None,
                index: 0,
                usage: None,
            },
            ChatChunk {
                content: Some("{\"ok\":true}".to_string()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
                index: 1,
                usage: None,
            },
        ]);
        let publisher = RecordingPublisher::default();
        let output = execute_llm_generation(
            &provider,
            &publisher,
            LlmGenerationRequest {
                request: ChatRequest::new(vec![ChatMessage::user("写作")], None, None),
                stream_id: "raw_json".to_string(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(output.content, "{\"ok\":true}");
        assert_eq!(publisher.text.lock().unwrap().concat(), "{\"ok\":true}");
        assert_eq!(publisher.reasoning.lock().unwrap().concat(), "先分析");
    }

    #[tokio::test]
    async fn publishes_task_cumulative_usage_without_changing_call_usage() {
        let provider = FakeProvider(vec![ChatChunk {
            content: Some("done".to_string()),
            reasoning_content: None,
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            index: 0,
            usage: Some(UsageInfo {
                prompt_tokens: 3,
                completion_tokens: 4,
                total_tokens: 7,
                prompt_cache_hit_tokens: None,
                prompt_cache_miss_tokens: Some(2),
                prompt_cache_write_tokens: None,
            }),
        }]);
        let publisher = RecordingPublisher::default();
        let output = execute_llm_generation(
            &provider,
            &publisher,
            LlmGenerationRequest {
                request: ChatRequest::new(vec![ChatMessage::user("continue")], None, None),
                stream_id: "raw_json".to_string(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: Some(GenerationUsage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                    prompt_cache_hit_tokens: Some(5),
                    prompt_cache_miss_tokens: None,
                    prompt_cache_write_tokens: None,
                }),
            },
        )
        .await
        .unwrap();

        assert_eq!(output.usage.unwrap().total_tokens, 7);
        assert_eq!(
            *publisher.usage.lock().unwrap(),
            Some(GenerationUsage {
                prompt_tokens: 13,
                completion_tokens: 24,
                total_tokens: 37,
                prompt_cache_hit_tokens: Some(5),
                prompt_cache_miss_tokens: Some(2),
                prompt_cache_write_tokens: None,
            })
        );
    }

    #[tokio::test]
    async fn waits_for_stream_terminal_after_finish_reason_chunk() {
        let provider = FakeProvider(vec![
            ChatChunk {
                content: Some("done".to_string()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: Some("stop".to_string()),
                index: 0,
                usage: None,
            },
            ChatChunk {
                content: None,
                reasoning_content: None,
                tool_calls: None,
                finish_reason: None,
                index: 1,
                usage: Some(UsageInfo {
                    prompt_tokens: 3,
                    completion_tokens: 4,
                    total_tokens: 7,
                    prompt_cache_hit_tokens: Some(2),
                    prompt_cache_miss_tokens: Some(1),
                    prompt_cache_write_tokens: Some(1),
                }),
            },
        ]);

        let output = execute_llm_generation(
            &provider,
            &RecordingPublisher::default(),
            LlmGenerationRequest {
                request: ChatRequest::new(vec![ChatMessage::user("continue")], None, None),
                stream_id: "main".to_string(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: None,
            },
        )
        .await
        .expect("finish_reason chunk must not end collection before the stream terminal");

        assert_eq!(output.content, "done");
        assert_eq!(output.finish_reason, GenerationFinishReason::Stop);
        assert_eq!(output.usage.expect("trailing usage").total_tokens, 7);
    }

    #[tokio::test]
    async fn provider_stream_error_preserves_the_root_cause() {
        let error = execute_llm_generation(
            &FailingProvider,
            &RecordingPublisher::default(),
            LlmGenerationRequest {
                request: ChatRequest::new(vec![ChatMessage::user("continue")], None, None),
                stream_id: "main".to_string(),
                cancellation: CancellationToken::new(),
                checkpoint: None,
                usage_base: None,
            },
        )
        .await
        .expect_err("provider stream failure must remain visible");

        assert_eq!(error.code.as_str(), "provider_job_failed");
        assert!(error.message.contains("模型流读取失败"));
        assert!(error.message.contains("上游在接收 128 bytes 后提前断流"));
    }
}
