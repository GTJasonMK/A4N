use crate::{
    AppState,
    actions::request_preview::{
        ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
    },
};
use a4n_llm_provider_contract::provider::ChatRequest;

pub(super) async fn optimization_analysis_request_preview(
    state: &AppState,
    request: ChatRequest,
) -> Result<ModelRequestPreview, String> {
    chat_status_preview_for_state(
        state,
        "optimization_analysis_agent_loop",
        "内容优化分析 Agent 请求预览",
        vec![chat_step(
            "optimization_analysis_step_1",
            "内容优化分析 Agent Loop 第 1 步",
            Some("content_optimization"),
            "ready",
            None,
            request,
        )],
        "llm_chat",
    )
    .await
}

pub(super) fn chat_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<&str>,
    state: impl Into<String>,
    note: Option<&str>,
    request: ChatRequest,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        key,
        title,
        prompt_name.map(ToString::to_string),
        state,
        note.map(ToString::to_string),
        Some(request),
    )
}
