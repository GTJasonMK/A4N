use std::time::Instant;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::UsageInfo;
use a4n_progress::StructuredJsonProgressTracker;

use super::super::contracts::report_video_prompt_progress;
use super::super::request::{ensure_video_output_not_truncated, ensure_video_prompt_not_cancelled};
use super::contracts::{StreamVideoPromptInput, StreamedVideoPromptResult};
use super::wait::{next_video_prompt_stream_item, open_video_prompt_stream};

pub(in crate::video::prompts::generation::llm_chat) async fn stream_chat_with_video_cancel(
    input: StreamVideoPromptInput<'_, '_, '_, '_>,
) -> Result<StreamedVideoPromptResult, AppError> {
    let StreamVideoPromptInput {
        service,
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
    let mut stream = open_video_prompt_stream(
        service.llm_backend(),
        request,
        cancel_token,
        step,
        progress,
        started,
    )
    .await?;
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
            Err(error) => {
                tracing::warn!(
                    target: "a4n::video_prompt",
                    step = step.message,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "视频提示词 LLM stream 读取失败"
                );
                return Err(error);
            }
        };
        if let Some(chunk_usage) = chunk.usage.clone() {
            usage = Some(chunk_usage);
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
            index = index.checked_add(1).ok_or_else(|| {
                AppError::InvalidArgument("视频提示词 stream chunk 序号超过 u32 上限".to_string())
            })?;
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
