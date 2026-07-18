mod contracts;
mod request;
mod streaming;

use a4n_core::error::AppError;

use self::request::{
    chat_with_video_cancel, ensure_video_output_not_truncated, log_video_prompt_usage,
};
use self::streaming::{StreamVideoPromptInput, stream_chat_with_video_cancel};
use super::super::video_llm_step_error;

pub(super) use contracts::{
    RenderedVideoPromptTrace, VideoPromptLlmCallTraceContext, VideoPromptRenderInput,
    VideoPromptStepProgress, report_video_prompt_progress,
};
pub(in crate::video) use request::build_video_prompt_request;
pub(super) use request::build_video_prompt_request_with_call_trace;

pub(super) async fn render_and_chat_trace_with_context(
    input: VideoPromptRenderInput<'_, '_, '_, '_>,
) -> Result<RenderedVideoPromptTrace, AppError> {
    let VideoPromptRenderInput {
        service,
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        llm_call_trace,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
    } = input;
    let mut request = build_video_prompt_request_with_call_trace(
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        llm_call_trace,
    )
    .await?;
    if chunks.is_some() {
        request = request.with_stream_usage();
        let trace_request = request.clone();
        let streamed = stream_chat_with_video_cancel(StreamVideoPromptInput {
            service,
            request,
            cancel_token,
            step,
            progress,
            chunks,
            prompt_name,
            structured_progress,
        })
        .await
        .map_err(|err| video_llm_step_error(prompt_name, err))?;
        log_video_prompt_usage(prompt_name, streamed.usage.as_ref(), true);
        contracts::report_video_prompt_usage(progress, step, streamed.usage.as_ref())?;
        return Ok(RenderedVideoPromptTrace {
            prompt_name: prompt_name.to_string(),
            request: trace_request,
            response_text: streamed.content,
            usage: streamed.usage,
            finish_reason: streamed.finish_reason,
        });
    }
    let trace_request = request.clone();
    let response = chat_with_video_cancel(service.llm.as_ref(), request, cancel_token)
        .await
        .map_err(|err| video_llm_step_error(prompt_name, err))?;
    ensure_video_output_not_truncated(prompt_name, response.finish_reason.as_deref())?;
    log_video_prompt_usage(prompt_name, response.usage.as_ref(), false);
    contracts::report_video_prompt_usage(progress, step, response.usage.as_ref())?;
    Ok(RenderedVideoPromptTrace {
        prompt_name: prompt_name.to_string(),
        request: trace_request,
        response_text: response.message.content,
        usage: response.usage,
        finish_reason: response.finish_reason,
    })
}
