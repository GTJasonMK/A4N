use crate::{
    AppState,
    actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state},
};

use super::super::{SendOptimizationMessageInput, preview::chat_step};

pub async fn preview_send_optimization_message_request(
    state: &AppState,
    user_id: i64,
    input: SendOptimizationMessageInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .build_send_user_message_preview_request_for_user_validated(
            user_id,
            input.session_id,
            &input.message,
            &input.permissions,
            prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())?;
    chat_status_preview_for_state(
        state,
        "optimization_chat_agent_loop",
        "内容优化对话 Agent 请求预览",
        vec![chat_step(
            "optimization_chat_step_1",
            "内容优化对话 Agent Loop 第 1 步",
            Some("content_optimization_chat"),
            "ready",
            None,
            request,
        )],
        "llm_chat",
    )
    .await
}
