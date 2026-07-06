use std::pin::Pin;

use a4n_core::error::AppError;
use a4n_engine::llm::provider::{ChatChunk, ChatRequest, LlmProvider, is_truncated_finish_reason};
use futures::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;

use crate::structured_progress::{
    ProgressSpan, StructuredJsonProgressSpec, StructuredJsonProgressTracker,
};

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

pub async fn chat_json_stream_with_progress(
    input: ChatJsonStreamInput<'_>,
    mut report_progress: impl FnMut(u32, &str) -> Result<(), String>,
) -> Result<String, String> {
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
    let mut saw_first_content = false;
    let mut last_progress_current = start;
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
            if let Some((_, tracker)) = tracker.as_mut() {
                for event in tracker.consume(delta) {
                    last_progress_current = last_progress_current.max(event.current);
                    report_progress(last_progress_current, &event.message)?;
                }
            }
        }
        if let Some(reason) = chunk.finish_reason.as_deref() {
            finish_reason = Some(reason.to_string());
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
    Ok(content)
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
