use std::collections::HashMap;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use a4n_domain::llm::llm_token_profiles;
use a4n_error::AppError;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::provider::{
    ChatRequest, ChatResponse, LlmProvider, LlmTraceContext, UsageInfo, is_truncated_finish_reason,
};

use a4n_llm::context::context_aware_system_user_request;
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::VideoPromptLlmCallTraceContext;

const VIDEO_PROMPT_WORKFLOW: &str = "video_prompt_generation";

async fn build_video_prompt_request(
    prompt_renderer: &dyn PromptRenderer,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
    manifest: LlmContextManifest,
    temperature: f32,
) -> Result<ChatRequest, AppError> {
    let system = prompt_renderer
        .render_prompt(prompt_name, variables, Some(project_id))
        .await?;
    Ok(context_aware_system_user_request(
        system,
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

pub(in crate::video::prompt_generation) async fn build_video_prompt_request_with_call_trace(
    prompt_renderer: &dyn PromptRenderer,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
    manifest: LlmContextManifest,
    temperature: f32,
    llm_call_trace: VideoPromptLlmCallTraceContext,
) -> Result<ChatRequest, AppError> {
    let mut request = build_video_prompt_request(
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        manifest,
        temperature,
    )
    .await?;
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

pub(super) fn ensure_video_prompt_not_cancelled(
    cancel_token: Option<&CancellationToken>,
) -> Result<(), AppError> {
    if cancel_token.is_some_and(CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    Ok(())
}

pub(super) fn ensure_video_output_not_truncated(
    prompt_name: &str,
    finish_reason: Option<&str>,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} 输出被 max_tokens 截断，未形成完整 JSON。当前使用 token 档位 {}，请在设置页调大“媒体提示词”输出上限，或减少本次输入/输出规模后重试。",
            llm_token_profiles::MEDIA_PROMPT
        )));
    }
    a4n_llm::provider::validate_finish_reason(finish_reason)
}

pub(super) fn log_video_prompt_usage(prompt_name: &str, usage: Option<&UsageInfo>, streamed: bool) {
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

pub(super) async fn chat_with_video_cancel(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChatResponse, AppError> {
    ensure_video_prompt_not_cancelled(cancel_token)?;
    let cancellation = cancel_token.cloned().unwrap_or_else(CancellationToken::new);
    crate::generation::stream::execute_llm_chat_response(llm, request, cancellation).await
}

#[cfg(test)]
mod tests {
    use super::{AppError, ensure_video_output_not_truncated, llm_token_profiles};

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
