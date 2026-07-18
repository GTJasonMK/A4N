use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse};

use crate::optimize::{
    OrchestratorContext,
    streaming::{OptimizeLlmStream, collect_streaming_chat_response},
    turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED, TURN_STATUS_FAILED},
};

pub(super) async fn call_patch_generation_llm(
    service: &OrchestratorContext,
    session_id: i64,
    turn_id: i64,
    request: ChatRequest,
    stream: &mut Option<OptimizeLlmStream<'_>>,
) -> Result<ChatResponse, AppError> {
    match collect_streaming_chat_response(service.llm.as_ref(), request, stream).await {
        Ok(response) => Ok(response),
        Err(err) => {
            service
                .append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("局部补丁模型调用失败：{err}"),
                    None,
                )
                .await?;
            service
                .fail_turn(turn_id, TURN_STATUS_FAILED, &err.to_string())
                .await?;
            service
                .save_agent_event(session_id, &format!("LLM 局部补丁调用失败：{err}"))
                .await?;
            Err(err)
        }
    }
}
