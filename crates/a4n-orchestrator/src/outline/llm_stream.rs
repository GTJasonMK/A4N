//! 大纲 generation 任务内的统一 LLM stream 上下文。
//!
//! 生产大纲 LLM 调用必须走 `execute_llm_generation`，禁止 `chat_llm` 完整响应。

use std::future::Future;

use a4n_error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse};
use tokio::task_local;

use crate::OrchestratorContext;
use crate::generation::stream::{TaskLlmStreamContext, execute_task_llm_chat_response};

task_local! {
    static OUTLINE_LLM_STREAM: OutlineLlmStreamContext;
}

pub(in crate::outline) type OutlineLlmStreamContext = TaskLlmStreamContext;

pub(in crate::outline) async fn with_outline_llm_stream<F, Fut, T>(
    context: OutlineLlmStreamContext,
    work: F,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    OUTLINE_LLM_STREAM.scope(context, Box::pin(work())).await
}

/// 在大纲 generation 任务上下文中执行统一 stream driver。
pub(in crate::outline) async fn execute_outline_llm_call(
    context: &OrchestratorContext,
    request: ChatRequest,
    call_key: &str,
) -> Result<ChatResponse, AppError> {
    let stream = OUTLINE_LLM_STREAM
        .try_with(|value| value.clone())
        .map_err(|_| AppError::InvalidState {
            current: "outline_llm_stream_missing".to_string(),
            operation: format!("大纲 LLM 调用 {call_key} 必须在 generation stream 上下文中执行"),
        })?;
    execute_task_llm_chat_response(context, &stream, request, call_key, "outline_generation").await
}
