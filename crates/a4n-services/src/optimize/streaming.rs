use a4n_core::error::AppError;
use a4n_engine::llm::provider::{
    ChatMessage, ChatRequest, ChatResponse, LlmProvider, is_truncated_finish_reason,
};
use futures::StreamExt;

pub type OptimizeProgress<'a> = &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);
pub type OptimizeLlmStream<'a> = &'a mut (dyn FnMut(&str, u32) -> Result<(), AppError> + Send);

pub(super) fn report_optimize_progress(
    progress: &mut Option<OptimizeProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message)?;
    }
    Ok(())
}

pub(super) async fn collect_streaming_chat_response(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    stream: &mut Option<OptimizeLlmStream<'_>>,
) -> Result<ChatResponse, AppError> {
    let mut chunks = llm.chat_stream(request).await?;
    let mut content = String::new();
    let mut chunk_index = 0;
    let mut finish_reason = None;

    while let Some(item) = chunks.next().await {
        let chunk = item?;
        if let Some(delta) = chunk.content.as_deref().filter(|delta| !delta.is_empty()) {
            content.push_str(delta);
            if let Some(stream) = stream.as_deref_mut() {
                stream(delta, chunk_index)?;
            }
            chunk_index += 1;
        }
        if chunk.finish_reason.is_some() {
            finish_reason = chunk.finish_reason;
            break;
        }
    }

    ensure_optimize_response_not_truncated(finish_reason.as_deref())?;

    Ok(ChatResponse {
        message: ChatMessage::assistant(content),
        usage: None,
        finish_reason,
    })
}

pub(super) fn ensure_optimize_response_not_truncated(
    finish_reason: Option<&str>,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(
            "内容优化模型输出达到当前 max_tokens 上限，返回 JSON 可能被截断；请在设置页调大对应输出档位，或减少本次优化上下文后重试".to_string(),
        ));
    }
    Ok(())
}
