//! 人物立绘 generation 任务内的统一 LLM stream 上下文。

use std::future::Future;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use tokio::task_local;

use crate::OrchestratorContext;
use crate::generation::stream::{TaskLlmStreamContext, execute_task_llm_chat_response};

task_local! {
    static PORTRAIT_LLM_STREAM: PortraitLlmStreamContext;
}

pub(in crate::portrait) type PortraitLlmStreamContext = TaskLlmStreamContext;

pub(in crate::portrait) async fn with_portrait_llm_stream<F, Fut, T>(
    context: PortraitLlmStreamContext,
    work: F,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    PORTRAIT_LLM_STREAM.scope(context, Box::pin(work())).await
}

pub(in crate::portrait) async fn execute_portrait_llm_call(
    context: &OrchestratorContext,
    request: ChatRequest,
    call_key: &str,
) -> Result<ChatResponse, AppError> {
    let stream = PORTRAIT_LLM_STREAM
        .try_with(|value| value.clone())
        .map_err(|_| AppError::InvalidState {
            current: "portrait_llm_stream_missing".to_string(),
            operation: format!(
                "人物立绘 LLM 调用 {call_key} 必须在 generation stream 上下文中执行"
            ),
        })?;
    execute_task_llm_chat_response(
        context,
        &stream,
        request,
        call_key,
        "portrait_generation",
        super::normalize_portrait_prompt_response,
    )
    .await
    .map(|(response, _)| response)
}
