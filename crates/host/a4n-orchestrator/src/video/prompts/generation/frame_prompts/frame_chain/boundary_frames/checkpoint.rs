use a4n_db::workflow;
use a4n_error::AppError;

use crate::video::prompts::generation::VideoPromptStepProgress;
use crate::video::prompts::generation::frame_prompts::{
    RenderOrRestoreJsonWithValidationInput, RenderedVideoPromptTrace,
    apply_boundary_frame_style_template, boundary_frame_expected_duration,
    boundary_frame_expected_segment_numbers, frame_prompt_progress_spec,
    parse_single_video_frame_prompt, render_or_restore_json_with_validation_with_context,
    serialize_video_value, validate_single_boundary_frame_prompt,
};
use crate::video::prompts::json::{VideoFramePrompt, VideoFramePromptDefaults};
use crate::workflow::workflow_checkpoint_contract_app_error;

use super::contracts::{BoundaryFrameCheckpointInput, PreparedBoundaryFrameRequest};

pub(in crate::video::prompts::generation::frame_prompts::frame_chain) async fn render_boundary_frame_checkpoint(
    input: BoundaryFrameCheckpointInput<'_, '_, '_>,
) -> Result<(RenderedVideoPromptTrace, VideoFramePrompt), AppError> {
    let BoundaryFrameCheckpointInput {
        service,
        spec,
        project_id,
        frame_number,
        workflow_run_id,
        prompt_renderer,
        variables,
        cancel_token,
        progress_span,
        generation_message,
        prepared_request,
        segment_plan,
        aspect_ratio,
        style,
        prompt_output_language,
        progress,
        chunks,
    } = input;
    let PreparedBoundaryFrameRequest {
        previous_frame_image_id,
        allowed_reference_ids,
        reference_image_ids,
        context_manifest,
        step_key,
        fingerprint_json,
    } = prepared_request;
    let frame_workflow_run_id =
        workflow::required_llm_call_workflow_run_id(workflow_run_id, &step_key)
            .map_err(workflow_checkpoint_contract_app_error)?;

    render_or_restore_json_with_validation_with_context(
        RenderOrRestoreJsonWithValidationInput {
            service,
            workflow_run_id: frame_workflow_run_id,
            call_key: step_key,
            call_order: 300 + frame_number,
            input_fingerprint_json: fingerprint_json,
            prompt_renderer,
            prompt_name: spec.frame_prompt_name,
            project_id,
            variables,
            context_manifest,
            temperature: 0.45,
            cancel_token,
            step: VideoPromptStepProgress::new(progress_span.start(), 100, generation_message)
                .with_span(progress_span),
            progress,
            chunks,
            structured_progress: Some(frame_prompt_progress_spec("边界帧", Some(1))),
            checkpoint_context: "边界帧提示词断点",
        },
        |raw| {
            let expected_segment_numbers =
                boundary_frame_expected_segment_numbers(segment_plan, frame_number)?;
            let default_duration_secs =
                boundary_frame_expected_duration(segment_plan, &expected_segment_numbers)?;
            let mut frame_prompt = parse_single_video_frame_prompt(
                raw,
                VideoFramePromptDefaults {
                    frame_number,
                    duration_secs: default_duration_secs,
                    aspect_ratio,
                    segment_numbers: expected_segment_numbers,
                    reference_image_ids: reference_image_ids.clone(),
                },
                prompt_output_language,
                frame_number,
                "边界帧提示词",
            )?;
            apply_boundary_frame_style_template(&mut frame_prompt, style, prompt_output_language);
            validate_single_boundary_frame_prompt(
                &frame_prompt,
                segment_plan,
                &allowed_reference_ids,
                previous_frame_image_id,
            )?;
            Ok(frame_prompt)
        },
        |frame_prompt| serialize_video_value(frame_prompt, "边界帧提示词断点"),
    )
    .await
}
