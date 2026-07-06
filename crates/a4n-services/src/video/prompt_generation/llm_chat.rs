use std::collections::HashMap;
use std::pin::Pin;
use std::time::{Duration, Instant};

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_engine::llm::provider::{
    ChatChunk, ChatRequest, ChatResponse, LlmProvider, LlmTraceContext, UsageInfo,
    is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
use futures::{Stream, StreamExt};
use serde_json::Value;
use tokio::time::{Instant as TokioInstant, MissedTickBehavior, interval_at};
use tokio_util::sync::CancellationToken;

use super::super::{VideoPromptChunk, VideoPromptProgress, video_llm_step_error};
use crate::generation::llm_usage_summary_text;
use crate::llm_context::{LlmContextManifest, context_aware_system_user_request};
use crate::structured_progress::{
    ProgressSpan, StructuredJsonProgressSpec, StructuredJsonProgressTracker,
};

const VIDEO_PROMPT_STREAM_HEARTBEAT: Duration = Duration::from_secs(8);
const VIDEO_PROMPT_WORKFLOW: &str = "video_prompt_generation";

#[derive(Clone, Copy)]
pub(super) struct VideoPromptStepProgress<'a> {
    pub(super) current: u32,
    pub(super) total: u32,
    pub(super) message: &'a str,
    pub(super) span: Option<ProgressSpan>,
}

#[derive(Debug, Clone)]
pub(super) struct RenderedVideoPromptTrace {
    pub(super) prompt_name: String,
    pub(super) request: ChatRequest,
    pub(super) response_text: String,
    pub(super) usage: Option<UsageInfo>,
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct VideoPromptLlmCallTraceContext {
    pub(super) workflow_run_id: i64,
    pub(super) call_key: String,
    pub(super) input_fingerprint_sha256: String,
}

struct StreamedVideoPromptResult {
    content: String,
    usage: Option<UsageInfo>,
    finish_reason: Option<String>,
}

pub(super) struct VideoPromptRenderInput<'a, 'step, 'progress, 'chunk> {
    pub(super) prompt_engine: &'a dyn PromptEngine,
    pub(super) llm: &'a dyn LlmProvider,
    pub(super) prompt_name: &'a str,
    pub(super) project_id: i64,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) context_manifest: LlmContextManifest,
    pub(super) temperature: f32,
    pub(super) llm_call_trace: VideoPromptLlmCallTraceContext,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) step: VideoPromptStepProgress<'step>,
    pub(super) progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub(super) chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
    pub(super) structured_progress: Option<StructuredJsonProgressSpec>,
}

impl<'a> VideoPromptStepProgress<'a> {
    pub(super) fn new(current: u32, total: u32, message: &'a str) -> Self {
        Self {
            current,
            total,
            message,
            span: None,
        }
    }

    pub(super) fn with_span(mut self, span: ProgressSpan) -> Self {
        self.span = Some(span);
        self
    }
}

pub(super) fn report_video_prompt_progress(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress.as_deref_mut() {
        report(current, total.max(1), message)?;
    }
    Ok(())
}

fn report_video_prompt_waiting(
    progress: &mut Option<VideoPromptProgress<'_>>,
    step: VideoPromptStepProgress<'_>,
    current: u32,
    elapsed: Duration,
    phase: &str,
) -> Result<(), AppError> {
    let elapsed_secs = elapsed.as_secs().max(1);
    let message = format!("{phase}：{}（已等待 {} 秒）", step.message, elapsed_secs);
    report_video_prompt_progress(progress, current, step.total, &message)
}

fn report_video_prompt_usage(
    progress: &mut Option<VideoPromptProgress<'_>>,
    step: VideoPromptStepProgress<'_>,
    usage: Option<&UsageInfo>,
) -> Result<(), AppError> {
    let message = format!(
        "模型请求完成：{}；{}",
        step.message,
        llm_usage_summary_text(usage)
    );
    report_video_prompt_progress(progress, step.current, step.total, &message)
}

pub(super) async fn render_and_chat_trace_with_context(
    input: VideoPromptRenderInput<'_, '_, '_, '_>,
) -> Result<RenderedVideoPromptTrace, AppError> {
    let VideoPromptRenderInput {
        prompt_engine,
        llm,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        llm_call_trace,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
    } = input;
    let mut request = build_video_prompt_request_with_call_trace(
        prompt_engine,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        llm_call_trace,
    )?;
    if chunks.is_some() {
        request = request.with_stream_usage();
        let trace_request = request.clone();
        let streamed = stream_chat_with_video_cancel(StreamVideoPromptInput {
            llm,
            request,
            cancel_token,
            step,
            progress,
            chunks,
            prompt_name,
            structured_progress,
        })
        .await
        .map_err(|err| video_llm_step_error(prompt_name, err))?;
        log_video_prompt_usage(prompt_name, streamed.usage.as_ref(), true);
        report_video_prompt_usage(progress, step, streamed.usage.as_ref())?;
        return Ok(RenderedVideoPromptTrace {
            prompt_name: prompt_name.to_string(),
            request: trace_request,
            response_text: streamed.content,
            usage: streamed.usage,
            finish_reason: streamed.finish_reason,
        });
    }
    let trace_request = request.clone();
    let resp = chat_with_video_cancel(llm, request, cancel_token)
        .await
        .map_err(|err| video_llm_step_error(prompt_name, err))?;
    ensure_video_output_not_truncated(prompt_name, resp.finish_reason.as_deref())?;
    log_video_prompt_usage(prompt_name, resp.usage.as_ref(), false);
    report_video_prompt_usage(progress, step, resp.usage.as_ref())?;
    Ok(RenderedVideoPromptTrace {
        prompt_name: prompt_name.to_string(),
        request: trace_request,
        response_text: resp.message.content,
        usage: resp.usage,
        finish_reason: resp.finish_reason,
    })
}

pub(in crate::video) fn build_video_prompt_request(
    prompt_engine: &dyn PromptEngine,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
    manifest: LlmContextManifest,
    temperature: f32,
) -> Result<ChatRequest, AppError> {
    let sys = prompt_engine.render(prompt_name, variables, Some(project_id))?;
    Ok(context_aware_system_user_request(
        sys,
        manifest,
        Some(temperature),
        None,
        Some(llm_token_profiles::MEDIA_PROMPT.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: VIDEO_PROMPT_WORKFLOW.to_string(),
        step_key: prompt_name.to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

pub(super) fn build_video_prompt_request_with_call_trace(
    prompt_engine: &dyn PromptEngine,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
    manifest: LlmContextManifest,
    temperature: f32,
    llm_call_trace: VideoPromptLlmCallTraceContext,
) -> Result<ChatRequest, AppError> {
    let mut request = build_video_prompt_request(
        prompt_engine,
        prompt_name,
        project_id,
        variables,
        manifest,
        temperature,
    )?;
    apply_video_prompt_llm_call_trace(prompt_name, &mut request, llm_call_trace)?;
    Ok(request)
}

fn apply_video_prompt_llm_call_trace(
    prompt_name: &str,
    request: &mut ChatRequest,
    llm_call_trace: VideoPromptLlmCallTraceContext,
) -> Result<(), AppError> {
    let trace_context = request
        .trace_context
        .as_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_llm_trace_context".to_string(),
            operation: format!("绑定视频提示词 LLM 调用上下文 {prompt_name}"),
        })?;
    trace_context.workflow_run_id = Some(llm_call_trace.workflow_run_id);
    trace_context.call_key = Some(llm_call_trace.call_key);
    trace_context.input_fingerprint_sha256 = Some(llm_call_trace.input_fingerprint_sha256);
    Ok(())
}

fn ensure_video_prompt_not_cancelled(
    cancel_token: Option<&CancellationToken>,
) -> Result<(), AppError> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

fn ensure_video_output_not_truncated(
    prompt_name: &str,
    finish_reason: Option<&str>,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} 输出被 max_tokens 截断，未形成完整 JSON。当前使用 token 档位 {}，请在设置页调大“媒体提示词”输出上限，或减少本次输入/输出规模后重试。",
            llm_token_profiles::MEDIA_PROMPT
        )));
    }
    Ok(())
}

fn log_video_prompt_usage(prompt_name: &str, usage: Option<&UsageInfo>, streamed: bool) {
    let Some(usage) = usage else {
        tracing::debug!(
            target: "a4n::video_prompt",
            prompt_name,
            streamed,
            "视频提示词 LLM 未返回 token usage"
        );
        return;
    };
    let cache_hit_tokens = usage.prompt_cache_hit_tokens.unwrap_or(0);
    let cache_miss_tokens = usage.prompt_cache_miss_tokens.unwrap_or(0);
    let cacheable_prompt_tokens = cache_hit_tokens + cache_miss_tokens;
    let cache_hit_ratio = if cacheable_prompt_tokens == 0 {
        None
    } else {
        Some(cache_hit_tokens as f64 / cacheable_prompt_tokens as f64)
    };
    tracing::info!(
        target: "a4n::video_prompt",
        prompt_name,
        streamed,
        prompt_tokens = usage.prompt_tokens,
        completion_tokens = usage.completion_tokens,
        total_tokens = usage.total_tokens,
        prompt_cache_hit_tokens = cache_hit_tokens,
        prompt_cache_miss_tokens = cache_miss_tokens,
        prompt_cache_hit_ratio = cache_hit_ratio,
        "视频提示词 LLM token usage"
    );
}

async fn chat_with_video_cancel(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChatResponse, AppError> {
    ensure_video_prompt_not_cancelled(cancel_token)?;
    match cancel_token {
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => Err(AppError::Cancelled),
                result = llm.chat(request) => result,
            }
        }
        None => llm.chat(request).await,
    }
}

async fn open_video_prompt_stream(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
    step: VideoPromptStepProgress<'_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    started: Instant,
) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError> {
    let mut request_future = Box::pin(llm.chat_stream(request));
    let mut heartbeat = interval_at(
        TokioInstant::now() + VIDEO_PROMPT_STREAM_HEARTBEAT,
        VIDEO_PROMPT_STREAM_HEARTBEAT,
    );
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        match cancel_token {
            Some(token) => {
                tokio::select! {
                    _ = token.cancelled() => return Err(AppError::Cancelled),
                    result = &mut request_future => return result,
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, step.current, started.elapsed(), "正在连接模型流")?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream 连接"
                        );
                    }
                }
            }
            None => {
                tokio::select! {
                    result = &mut request_future => return result,
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, step.current, started.elapsed(), "正在连接模型流")?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream 连接"
                        );
                    }
                }
            }
        }
    }
}

async fn next_video_prompt_stream_item(
    stream: &mut Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
    cancel_token: Option<&CancellationToken>,
    step: VideoPromptStepProgress<'_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    started: Instant,
    phase: &str,
    current: u32,
) -> Result<Option<Result<ChatChunk, AppError>>, AppError> {
    let mut heartbeat = interval_at(
        TokioInstant::now() + VIDEO_PROMPT_STREAM_HEARTBEAT,
        VIDEO_PROMPT_STREAM_HEARTBEAT,
    );
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        match cancel_token {
            Some(token) => {
                tokio::select! {
                    _ = token.cancelled() => return Err(AppError::Cancelled),
                    item = stream.next() => return Ok(item),
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, current, started.elapsed(), phase)?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream chunk"
                        );
                    }
                }
            }
            None => {
                tokio::select! {
                    item = stream.next() => return Ok(item),
                    _ = heartbeat.tick() => {
                        report_video_prompt_waiting(progress, step, current, started.elapsed(), phase)?;
                        tracing::debug!(
                            target: "a4n::video_prompt",
                            step = step.message,
                            duration_ms = started.elapsed().as_millis(),
                            "等待视频提示词 LLM stream chunk"
                        );
                    }
                }
            }
        }
    }
}

struct StreamVideoPromptInput<'a, 'step, 'progress, 'chunk> {
    llm: &'a dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&'a CancellationToken>,
    step: VideoPromptStepProgress<'step>,
    progress: &'a mut Option<VideoPromptProgress<'progress>>,
    chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
    prompt_name: &'a str,
    structured_progress: Option<StructuredJsonProgressSpec>,
}

async fn stream_chat_with_video_cancel(
    input: StreamVideoPromptInput<'_, '_, '_, '_>,
) -> Result<StreamedVideoPromptResult, AppError> {
    let StreamVideoPromptInput {
        llm,
        request,
        cancel_token,
        step,
        progress,
        chunks,
        prompt_name,
        structured_progress,
    } = input;
    ensure_video_prompt_not_cancelled(cancel_token)?;
    let started = Instant::now();
    tracing::info!(
        target: "a4n::video_prompt",
        step = step.message,
        "视频提示词 LLM stream 开始"
    );
    let mut stream =
        open_video_prompt_stream(llm, request, cancel_token, step, progress, started).await?;
    tracing::info!(
        target: "a4n::video_prompt",
        step = step.message,
        duration_ms = started.elapsed().as_millis(),
        "视频提示词 LLM stream 已连接"
    );
    report_video_prompt_progress(
        progress,
        step.current,
        step.total,
        &format!("模型流已连接：{}，正在等待输出", step.message),
    )?;
    let mut content = String::new();
    let mut usage = None::<UsageInfo>;
    let mut index = 0_u32;
    let mut saw_first_chunk = false;
    let mut finish_reason = None::<String>;
    let mut last_progress_current = step.current;
    let mut structured_tracker = structured_progress
        .zip(step.span)
        .map(|(spec, span)| StructuredJsonProgressTracker::new(spec, span));
    loop {
        let waiting_phase = if saw_first_chunk {
            "等待模型继续输出"
        } else {
            "模型已连接，等待首个流式输出"
        };
        let item = next_video_prompt_stream_item(
            &mut stream,
            cancel_token,
            step,
            progress,
            started,
            waiting_phase,
            last_progress_current,
        )
        .await?;
        let Some(item) = item else {
            break;
        };
        let chunk = match item {
            Ok(chunk) => chunk,
            Err(err) => {
                tracing::warn!(
                    target: "a4n::video_prompt",
                    step = step.message,
                    duration_ms = started.elapsed().as_millis(),
                    error = %err,
                    "视频提示词 LLM stream 读取失败"
                );
                return Err(err);
            }
        };
        if let Some(chunk_usage) = chunk.usage.clone() {
            usage = Some(chunk_usage);
        }
        if let Some(delta) = chunk
            .reasoning_content
            .as_deref()
            .filter(|text| !text.is_empty())
        {
            if !saw_first_chunk {
                saw_first_chunk = true;
                last_progress_current = last_progress_current.max(step.current);
                report_video_prompt_progress(
                    progress,
                    step.current,
                    step.total,
                    &format!("模型已开始输出：{}", step.message),
                )?;
                tracing::info!(
                    target: "a4n::video_prompt",
                    step = step.message,
                    duration_ms = started.elapsed().as_millis(),
                    "视频提示词 LLM stream 收到首个可显示 chunk"
                );
            }
            if let Some(report) = chunks.as_deref_mut() {
                report(delta, index);
            }
            index = index.saturating_add(1);
        }
        if let Some(delta) = chunk.content.as_deref().filter(|text| !text.is_empty()) {
            if !saw_first_chunk {
                saw_first_chunk = true;
                last_progress_current = last_progress_current.max(step.current);
                report_video_prompt_progress(
                    progress,
                    step.current,
                    step.total,
                    &format!("模型已开始输出：{}", step.message),
                )?;
                tracing::info!(
                    target: "a4n::video_prompt",
                    step = step.message,
                    duration_ms = started.elapsed().as_millis(),
                    "视频提示词 LLM stream 收到首个可显示 chunk"
                );
            }
            if let Some(report) = chunks.as_deref_mut() {
                report(delta, index);
            }
            index = index.saturating_add(1);
            content.push_str(delta);
            if let Some(tracker) = structured_tracker.as_mut() {
                for event in tracker.consume(delta) {
                    last_progress_current = last_progress_current.max(event.current);
                    report_video_prompt_progress(
                        progress,
                        last_progress_current,
                        event.total,
                        &event.message,
                    )?;
                }
            }
        }
        if let Some(reason) = chunk.finish_reason.as_deref() {
            finish_reason = Some(reason.to_string());
        }
    }
    tracing::info!(
        target: "a4n::video_prompt",
        step = step.message,
        duration_ms = started.elapsed().as_millis(),
        visible_chunk_count = index,
        response_chars = content.chars().count(),
        "视频提示词 LLM stream 结束"
    );
    ensure_video_output_not_truncated(prompt_name, finish_reason.as_deref())?;
    Ok(StreamedVideoPromptResult {
        content,
        usage,
        finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_waiting_progress_at_current_floor() {
        let mut calls = Vec::<(u32, u32, String)>::new();
        {
            let mut progress_fn =
                |current: u32, total: u32, message: &str| -> Result<(), AppError> {
                    calls.push((current, total, message.to_string()));
                    Ok(())
                };
            let mut progress: Option<VideoPromptProgress<'_>> =
                Some(&mut progress_fn as VideoPromptProgress<'_>);

            report_video_prompt_waiting(
                &mut progress,
                VideoPromptStepProgress::new(20, 100, "生成中"),
                57,
                Duration::from_secs(9),
                "等待模型继续输出",
            )
            .expect("waiting progress should report");
        }

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 57);
        assert_eq!(calls[0].1, 100);
        assert!(calls[0].2.contains("等待模型继续输出：生成中"));
    }

    #[test]
    fn detects_video_output_truncated_by_max_tokens() {
        let err = ensure_video_output_not_truncated("chapter_video_prompt", Some("max_tokens"))
            .expect_err("max_tokens finish reason should be rejected");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message)
                if message.contains("chapter_video_prompt 输出被 max_tokens 截断")
                    && message.contains(llm_token_profiles::MEDIA_PROMPT)
        ));
    }
}
