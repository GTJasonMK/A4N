use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use a4n_llm_context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

use super::{
    CompletedVideoLlmCheckpointInput, FailedVideoLlmCheckpointInput, NewChapterVideoPrompt,
    OrchestratorContext, PromptWorkflowMode, RenderedVideoPromptTrace, StructuredJsonProgressSpec,
    VideoPromptChunk, VideoPromptGenerationTrace, VideoPromptProgress, VideoPromptRenderInput,
    VideoPromptStepProgress, build_video_prompt_request_with_call_trace,
    render_and_chat_trace_with_context, report_video_prompt_progress,
    save_completed_video_llm_checkpoint, save_failed_video_llm_checkpoint,
    video_llm_call_trace_context, video_llm_validation_error_with_diagnostic,
};

async fn validate_video_trace<T>(
    service: &OrchestratorContext,
    trace: &mut RenderedVideoPromptTrace,
    cancel_token: Option<&CancellationToken>,
    validate: impl Fn(&str) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let initial = a4n_llm_provider_contract::provider::ChatResponse {
        message: a4n_llm_provider_contract::provider::ChatMessage::assistant(&trace.response_text),
        usage: trace.usage.clone(),
        finish_reason: trace.finish_reason.clone(),
        reasoning_content: None,
    };
    let (response, parsed) = service
        .validate_llm_response(
            &trace.request,
            initial,
            &trace.prompt_name,
            cancel_token,
            validate,
        )
        .await?;
    trace.response_text = response.message.content;
    trace.usage = response.usage;
    trace.finish_reason = response.finish_reason;
    Ok(parsed)
}

pub(super) fn attach_generation_trace_to_draft(
    drafts: Vec<NewChapterVideoPrompt>,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    shot_number: i32,
    trace: VideoPromptGenerationTrace,
) -> Vec<NewChapterVideoPrompt> {
    drafts
        .into_iter()
        .map(|draft| {
            if draft.matches_identity(workflow_mode, prompt_type, shot_number) {
                draft.with_generation_trace(trace.clone())
            } else {
                draft
            }
        })
        .collect()
}

pub(super) fn attach_generation_trace_to_all_drafts(
    drafts: Vec<NewChapterVideoPrompt>,
    trace: VideoPromptGenerationTrace,
) -> Vec<NewChapterVideoPrompt> {
    drafts
        .into_iter()
        .map(|draft| draft.with_generation_trace(trace.clone()))
        .collect()
}

pub(super) fn video_prompt_generation_trace(
    trace: RenderedVideoPromptTrace,
    workflow_run_id: Option<i64>,
) -> VideoPromptGenerationTrace {
    VideoPromptGenerationTrace {
        prompt_name: trace.prompt_name,
        request: trace.request,
        response_text: trace.response_text,
        workflow_run_id,
        usage: trace.usage,
    }
}

pub(super) async fn render_json_with_validation_with_context<T, F>(
    input: RenderOrRestoreJsonWithValidationInput<'_, '_, '_, '_>,
    parse_and_validate: F,
) -> Result<(RenderedVideoPromptTrace, T), AppError>
where
    F: Fn(&str) -> Result<T, AppError>,
{
    let RenderOrRestoreJsonWithValidationInput {
        service,
        workflow_run_id,
        call_key,
        call_order,
        input_fingerprint_json,
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
        checkpoint_context: _,
    } = input;
    let mut trace = render_and_chat_trace_with_context(VideoPromptRenderInput {
        service,
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        llm_call_trace: video_llm_call_trace_context(
            workflow_run_id,
            &call_key,
            &input_fingerprint_json,
        )?,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
    })
    .await?;
    report_video_prompt_progress(progress, step.current, step.total, "正在校验结构化视频输出")?;
    let parsed =
        match validate_video_trace(service, &mut trace, cancel_token, parse_and_validate).await {
            Ok(parsed) => parsed,
            Err(err) => {
                let diagnostic = save_failed_video_llm_checkpoint(
                    service,
                    FailedVideoLlmCheckpointInput {
                        workflow_run_id,
                        call_key,
                        call_order,
                        input_fingerprint_json,
                        prompt_name,
                        request: &trace.request,
                        response_content: Some(trace.response_text.as_str()),
                        error_message: err.to_string(),
                        source: "llm_response_validation",
                    },
                )
                .await;
                return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
            }
        };
    Ok((trace, parsed))
}

pub(super) struct RenderOrRestoreJsonWithValidationInput<'a, 'step, 'progress, 'chunk> {
    pub service: &'a OrchestratorContext,
    pub workflow_run_id: i64,
    pub call_key: String,
    pub call_order: i32,
    pub input_fingerprint_json: Value,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub prompt_name: &'a str,
    pub project_id: i64,
    pub variables: &'a HashMap<String, Value>,
    pub context_manifest: LlmContextManifest,
    pub temperature: f32,
    pub cancel_token: Option<&'a CancellationToken>,
    pub step: VideoPromptStepProgress<'step>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
    pub structured_progress: Option<StructuredJsonProgressSpec>,
    pub checkpoint_context: &'a str,
}

pub(super) async fn render_or_restore_json_with_validation_with_context<T, F, G>(
    input: RenderOrRestoreJsonWithValidationInput<'_, '_, '_, '_>,
    parse_and_validate: F,
    checkpoint_parsed_output: G,
) -> Result<(RenderedVideoPromptTrace, T), AppError>
where
    F: Fn(&str) -> Result<T, AppError>,
    G: Fn(&T) -> Result<Value, AppError>,
{
    let RenderOrRestoreJsonWithValidationInput {
        service,
        workflow_run_id,
        call_key,
        call_order,
        input_fingerprint_json,
        prompt_renderer,
        prompt_name,
        project_id,
        variables,
        context_manifest,
        temperature,
        cancel_token,
        step,
        progress,
        chunks,
        structured_progress,
        checkpoint_context,
    } = input;
    let llm_call_trace =
        video_llm_call_trace_context(workflow_run_id, &call_key, &input_fingerprint_json)?;
    if let Some(checkpoint) = service
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &input_fingerprint_json)
        .await?
    {
        let response_content = checkpoint.response_content.clone();
        let request = build_video_prompt_request_with_call_trace(
            prompt_renderer,
            prompt_name,
            project_id,
            variables,
            context_manifest.clone(),
            temperature,
            llm_call_trace.clone(),
        )
        .await?;
        let parsed = match parse_and_validate(&response_content) {
            Ok(parsed) => parsed,
            Err(err) => {
                let diagnostic = save_failed_video_llm_checkpoint(
                    service,
                    FailedVideoLlmCheckpointInput {
                        workflow_run_id,
                        call_key,
                        call_order,
                        input_fingerprint_json,
                        prompt_name,
                        request: &request,
                        response_content: Some(response_content.as_str()),
                        error_message: err.to_string(),
                        source: "completed_checkpoint_restore",
                    },
                )
                .await;
                return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
            }
        };
        let parsed_output = checkpoint_parsed_output(&parsed)?;
        if let Err(err) = ensure_checkpoint_parsed_output_matches(
            &checkpoint.parsed_output,
            &parsed_output,
            checkpoint_context,
        ) {
            let diagnostic = save_failed_video_llm_checkpoint(
                service,
                FailedVideoLlmCheckpointInput {
                    workflow_run_id,
                    call_key,
                    call_order,
                    input_fingerprint_json,
                    prompt_name,
                    request: &request,
                    response_content: Some(response_content.as_str()),
                    error_message: err.to_string(),
                    source: "completed_checkpoint_restore",
                },
            )
            .await;
            return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
        }
        let usage = checkpoint
            .usage
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| {
                AppError::InvalidArgument(format!("{checkpoint_context}.usage 解析失败: {err}"))
            })?;
        let finish_reason = checkpoint.finish_reason;
        report_video_prompt_progress(
            progress,
            step.current,
            step.total,
            &format!("{}（已复用调用级断点）", step.message),
        )?;
        return Ok((
            RenderedVideoPromptTrace {
                prompt_name: prompt_name.to_string(),
                request,
                response_text: response_content,
                usage,
                finish_reason,
            },
            parsed,
        ));
    }

    let mut trace = render_and_chat_trace_with_context(VideoPromptRenderInput {
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
    })
    .await?;
    report_video_prompt_progress(progress, step.current, step.total, "正在校验结构化视频输出")?;
    let parsed =
        match validate_video_trace(service, &mut trace, cancel_token, parse_and_validate).await {
            Ok(parsed) => parsed,
            Err(err) => {
                let diagnostic = save_failed_video_llm_checkpoint(
                    service,
                    FailedVideoLlmCheckpointInput {
                        workflow_run_id,
                        call_key,
                        call_order,
                        input_fingerprint_json,
                        prompt_name,
                        request: &trace.request,
                        response_content: Some(trace.response_text.as_str()),
                        error_message: err.to_string(),
                        source: "llm_response_validation",
                    },
                )
                .await;
                return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
            }
        };
    let parsed_output = checkpoint_parsed_output(&parsed)?;
    save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
        service,
        workflow_run_id,
        step_key: call_key,
        step_order: call_order,
        input_fingerprint_json,
        trace: &trace,
        parsed_output,
        artifacts_json: None,
    })
    .await?;
    Ok((trace, parsed))
}
