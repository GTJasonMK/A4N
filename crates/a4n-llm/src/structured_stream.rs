use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_core::structured_progress::{
    ProgressSpan, StructuredJsonProgressSpec, StructuredJsonProgressTracker,
};
use futures::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::provider::{ChatChunk, ChatRequest, LlmProvider, UsageInfo, is_truncated_finish_reason};

const STREAM_PROGRESS_HEARTBEAT_CHARS: usize = 1_024;

#[derive(Debug, Clone, Copy)]
pub struct StructuredLlmProgress {
    pub spec: StructuredJsonProgressSpec,
    pub span: ProgressSpan,
}

pub struct ChatJsonStreamInput<'a> {
    pub llm: &'a dyn LlmProvider,
    pub request: ChatRequest,
    pub prompt_name: &'a str,
    pub truncated_message: &'a str,
    pub structured_progress: Option<StructuredLlmProgress>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub cancelled_message: &'a str,
}

#[derive(Debug, Clone)]
pub struct StructuredLlmStreamResult {
    pub response_content: String,
    pub usage: Option<UsageInfo>,
    pub finish_reason: Option<String>,
}

pub async fn chat_json_stream_with_progress(
    input: ChatJsonStreamInput<'_>,
    report_progress: impl FnMut(u32, &str) -> Result<(), String>,
) -> Result<String, String> {
    chat_json_stream_result_with_progress(input, report_progress)
        .await
        .map(|result| result.response_content)
}

pub async fn chat_json_stream_result_with_progress(
    input: ChatJsonStreamInput<'_>,
    report_progress: impl FnMut(u32, &str) -> Result<(), String>,
) -> Result<StructuredLlmStreamResult, String> {
    chat_json_stream_result_with_sink(input, report_progress, None).await
}

/// 与 [`chat_json_stream_result_with_progress`] 相同，但额外把原始文本增量交给 `on_delta`。
///
/// 结构化里程碑与原始 delta 是同一个 token 流的两种投影：前者回答「进行到哪了」，
/// 后者回答「还活着」。二者来自同一次 `chat_stream`，不会重复请求模型。
///
/// `on_delta` 只做转发，不得阻塞或返回错误——活体信号的失败不应中断生成。
pub async fn chat_json_stream_result_with_sink(
    input: ChatJsonStreamInput<'_>,
    mut report_progress: impl FnMut(u32, &str) -> Result<(), String>,
    mut on_delta: Option<&mut (dyn FnMut(&str) + Send)>,
) -> Result<StructuredLlmStreamResult, String> {
    let ChatJsonStreamInput {
        llm,
        request,
        prompt_name,
        truncated_message,
        structured_progress,
        cancel_token,
        cancelled_message,
    } = input;
    let start = structured_progress
        .map(|progress| progress.span.start())
        .unwrap_or(0);
    report_progress(start, &format!("正在连接模型：{prompt_name}"))?;
    check_cancelled(cancel_token, cancelled_message)?;
    let mut stream = llm
        .chat_stream(request)
        .await
        .map_err(|err| format!("{prompt_name} 模型流建立失败: {err}"))?;
    report_progress(start, &format!("模型流已连接：{prompt_name}，正在等待输出"))?;

    let mut content = String::new();
    let mut finish_reason = None::<String>;
    let mut usage = None;
    let mut saw_first_content = false;
    let mut last_progress_current = start;
    let mut next_heartbeat_chars = STREAM_PROGRESS_HEARTBEAT_CHARS;
    let mut tracker = structured_progress.map(|progress| {
        (
            progress.span,
            StructuredJsonProgressTracker::new(progress.spec, progress.span),
        )
    });

    loop {
        let item = next_stream_item(&mut stream, cancel_token, cancelled_message).await?;
        let Some(item) = item else {
            break;
        };
        let chunk = item.map_err(|err| format!("{prompt_name} 模型流读取失败: {err}"))?;
        if let Some(delta) = chunk.content.as_deref().filter(|text| !text.is_empty()) {
            if !saw_first_content {
                saw_first_content = true;
                report_progress(
                    last_progress_current,
                    &format!("模型已开始输出：{prompt_name}"),
                )?;
            }
            content.push_str(delta);
            // 活体信号：与下面的结构化里程碑取自同一个 delta，互不替代。
            if let Some(sink) = on_delta.as_mut() {
                sink(delta);
            }
            if let Some((_, tracker)) = tracker.as_mut() {
                for event in tracker.consume(delta) {
                    last_progress_current = last_progress_current.max(event.current);
                    report_progress(last_progress_current, &event.message)?;
                }
            }
            if let Some((span, _)) = tracker.as_ref()
                && content.len() >= next_heartbeat_chars
            {
                let heartbeat_limit = span.end().saturating_sub(1);
                let heartbeat_current =
                    last_progress_current.saturating_add(1).min(heartbeat_limit);
                if heartbeat_current > last_progress_current {
                    last_progress_current = heartbeat_current;
                    report_progress(
                        last_progress_current,
                        &format!(
                            "模型持续输出：{prompt_name}，已接收约 {} 个字符",
                            content.len()
                        ),
                    )?;
                }
                next_heartbeat_chars = content
                    .len()
                    .saturating_add(STREAM_PROGRESS_HEARTBEAT_CHARS);
            }
        }
        if let Some(reason) = chunk.finish_reason.as_deref() {
            finish_reason = Some(reason.to_string());
        }
        if chunk.usage.is_some() {
            usage = chunk.usage;
        }
    }

    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(truncated_message.to_string());
    }
    if let Some((span, _)) = tracker {
        report_progress(
            span.end(),
            &format!("{prompt_name} 输出完成，正在解析 JSON"),
        )?;
    }
    Ok(StructuredLlmStreamResult {
        response_content: content,
        usage,
        finish_reason,
    })
}

async fn next_stream_item(
    stream: &mut Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>,
    cancel_token: Option<&CancellationToken>,
    cancelled_message: &str,
) -> Result<Option<Result<ChatChunk, AppError>>, String> {
    check_cancelled(cancel_token, cancelled_message)?;
    if let Some(cancel_token) = cancel_token {
        return tokio::select! {
            _ = cancel_token.cancelled() => Err(cancelled_message.to_string()),
            item = stream.next() => Ok(item),
        };
    }
    Ok(stream.next().await)
}

fn check_cancelled(
    cancel_token: Option<&CancellationToken>,
    cancelled_message: &str,
) -> Result<(), String> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        Err(cancelled_message.to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
