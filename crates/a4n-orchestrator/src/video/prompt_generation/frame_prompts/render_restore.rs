use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use a4n_llm::context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;

use super::{
    CompletedVideoLlmCheckpointInput, FailedVideoLlmCheckpointInput, NewChapterVideoPrompt,
    OrchestratorContext, PromptWorkflowMode, RenderedVideoPromptTrace, StructuredJsonProgressSpec,
    VideoPromptChunk, VideoPromptGenerationTrace, VideoPromptProgress, VideoPromptRenderInput,
    VideoPromptStepProgress, build_video_prompt_request_with_call_trace, checkpoint_finish_reason,
    checkpoint_response_content, checkpoint_usage, ensure_checkpoint_parsed_output,
    load_completed_video_checkpoint, render_and_chat_trace_with_context,
    report_video_prompt_progress, save_completed_video_llm_checkpoint,
    save_failed_video_llm_checkpoint, video_llm_call_trace_context,
    video_llm_validation_error_with_diagnostic,
};
use crate::structured_llm::{
    StructuredRepairCheckpointContext, StructuredValueRepairInput, parse_strict_json_response,
};
use crate::video::prompt_json::video_llm_output_schema;

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
    input: VideoPromptRenderInput<'_, '_, '_, '_>,
    repair_checkpoint: StructuredRepairCheckpointContext,
    parse_and_validate: F,
) -> Result<(RenderedVideoPromptTrace, T, Value), AppError>
where
    F: Fn(&str) -> Result<T, AppError>,
{
    let service = input.service;
    let cancel_token = input.cancel_token;
    let trace = render_and_chat_trace_with_context(input).await?;
    let parse_output = |value: &Value| parse_and_validate(&serde_json::to_string(value)?);
    let repaired = service
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: trace.request.clone(),
            raw_response: &trace.response_text,
            label: &trace.prompt_name,
            schema: video_llm_output_schema(&trace.prompt_name)?,
            checkpoint: Some(repair_checkpoint),
            cancel_token,
            progress: None,
            parse_output: &parse_output,
        })
        .await?;
    Ok((trace, repaired.output, repaired.final_value))
}

pub(in crate::video::prompt_generation::frame_prompts) struct RenderOrRestoreJsonWithValidationInput<
    'a,
    'step,
    'progress,
    'chunk,
> {
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

pub(in crate::video::prompt_generation::frame_prompts) struct RawVideoJsonResult {
    pub trace: RenderedVideoPromptTrace,
    pub value: Value,
    pub restored: bool,
}

pub(in crate::video::prompt_generation::frame_prompts) async fn render_or_restore_raw_video_json(
    input: RenderOrRestoreJsonWithValidationInput<'_, '_, '_, '_>,
) -> Result<RawVideoJsonResult, AppError> {
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
    if let Some(checkpoint) = load_completed_video_checkpoint(
        service,
        workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )
    .await?
    {
        let response_content =
            checkpoint_response_content(&checkpoint, checkpoint_context)?.to_string();
        let request = build_video_prompt_request_with_call_trace(
            prompt_renderer,
            prompt_name,
            project_id,
            variables,
            context_manifest,
            temperature,
            llm_call_trace,
        )
        .await?;
        let value = match checkpoint.get("structured_response") {
            Some(value) => value.clone(),
            None => parse_strict_json_response::<Value>(&response_content, checkpoint_context)?,
        };
        ensure_checkpoint_parsed_output(&checkpoint, &value, checkpoint_context)?;
        report_video_prompt_progress(
            progress,
            step.current,
            step.total,
            &format!("{}（已复用调用级断点）", step.message),
        )?;
        return Ok(RawVideoJsonResult {
            trace: RenderedVideoPromptTrace {
                prompt_name: prompt_name.to_string(),
                request,
                response_text: response_content,
                usage: checkpoint_usage(&checkpoint, checkpoint_context)?,
                finish_reason: checkpoint_finish_reason(&checkpoint, checkpoint_context)?,
            },
            value,
            restored: true,
        });
    }

    let trace = render_and_chat_trace_with_context(VideoPromptRenderInput {
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
    let value = match parse_strict_json_response::<Value>(&trace.response_text, prompt_name) {
        Ok(value) => value,
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
                    source: "strict_json_validation",
                },
            )
            .await;
            return Err(video_llm_validation_error_with_diagnostic(err, diagnostic));
        }
    };
    Ok(RawVideoJsonResult {
        trace,
        value,
        restored: false,
    })
}

pub(in crate::video::prompt_generation::frame_prompts) async fn render_or_restore_json_with_validation_with_context<
    T,
    F,
    G,
>(
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
    if let Some(checkpoint) = load_completed_video_checkpoint(
        service,
        workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )
    .await?
    {
        let response_content =
            checkpoint_response_content(&checkpoint, checkpoint_context)?.to_string();
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
        let validation_content = checkpoint
            .get("structured_response")
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_else(|| response_content.clone());
        let parsed = match parse_and_validate(&validation_content) {
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
        if let Err(err) =
            ensure_checkpoint_parsed_output(&checkpoint, &parsed_output, checkpoint_context)
        {
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
        let usage = checkpoint_usage(&checkpoint, checkpoint_context)?;
        let finish_reason = checkpoint_finish_reason(&checkpoint, checkpoint_context)?;
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

    let trace = render_and_chat_trace_with_context(VideoPromptRenderInput {
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
    let parse_output = |value: &Value| parse_and_validate(&serde_json::to_string(value)?);
    report_video_prompt_progress(progress, step.current, step.total, "正在校验结构化视频输出")?;
    let repaired = service
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request: trace.request.clone(),
            raw_response: &trace.response_text,
            label: prompt_name,
            schema: video_llm_output_schema(prompt_name)?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id,
                base_call_key: call_key.clone(),
                call_order,
            }),
            cancel_token,
            progress: None,
            parse_output: &parse_output,
        })
        .await;
    let parsed = match repaired {
        Ok(repaired) => repaired,
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
    let parsed_output = checkpoint_parsed_output(&parsed.output)?;
    save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
        service,
        workflow_run_id,
        step_key: call_key,
        step_order: call_order,
        input_fingerprint_json,
        trace: &trace,
        parsed_output,
        extra_json: json!({
            "structured_response": parsed.final_value,
        }),
        artifacts_json: Some(json!({
            "structured_repair": parsed.artifact_summary(),
        })),
    })
    .await?;
    Ok((trace, parsed.output))
}
