use a4n_db::workflow;
use serde_json::json;

use crate::video::prompt_generation::frame_prompts::{
    AppError, CompletedVideoLlmCheckpointInput, FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION,
    ProgressSpan, RenderOrRestoreJsonWithValidationInput, VideoPromptChunk, VideoPromptProgress,
    VideoPromptStepProgress, build_final_video_context_manifest,
    parse_single_generated_video_prompt_shot, render_or_restore_raw_video_json,
    save_completed_video_llm_checkpoint, video_prompt_generation_trace, video_prompt_progress_spec,
    video_stage_fingerprint,
};
use crate::workflow::workflow_checkpoint_contract_app_error;

use super::prompts::build_video_prompt_user_for_spec;
use super::targeted_repair::repair_invalid_video_prompt_fields;
use super::{ExecutedVideoPromptUnit, VideoPromptUnitCheckpointContext};

pub(super) async fn generate_video_prompt_draft(
    context: VideoPromptUnitCheckpointContext<'_, '_>,
    progress_span: ProgressSpan,
    generation_message: &str,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<ExecutedVideoPromptUnit, AppError> {
    let prompt_user =
        build_video_prompt_user_for_spec(context.spec, context.final_prompt_context).await?;
    let prompt_context_manifest = build_final_video_context_manifest(
        context.prompt_renderer,
        context.project_id,
        context.vars,
        context.source,
        &[
            "chapter_video_segment_plan",
            context.spec.frame_prompt_name,
            context.spec.video_prompt_name,
        ],
        prompt_user,
    )
    .await?;
    let draft_step_key = format!(
        "{}:shot:{}:draft",
        context.spec.video_prompt_name, context.shot_number
    );
    let draft_fingerprint_json = video_stage_fingerprint(
        context.spec.video_prompt_name,
        &prompt_context_manifest,
        json!({
            "project_id": context.project_id,
            "chapter_id": context.chapter_id,
            "shot_number": context.shot_number,
            "prompt_type": "video_prompt_draft",
            "output_contract_version": FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION,
        }),
    );
    let draft_workflow_run_id =
        workflow::required_llm_call_workflow_run_id(context.workflow_run_id, &draft_step_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
    let call_order = 400 + context.shot_number;
    let raw = render_or_restore_raw_video_json(RenderOrRestoreJsonWithValidationInput {
        service: context.service,
        workflow_run_id: draft_workflow_run_id,
        call_key: draft_step_key.clone(),
        call_order,
        input_fingerprint_json: draft_fingerprint_json.clone(),
        prompt_renderer: context.prompt_renderer,
        prompt_name: context.spec.video_prompt_name,
        project_id: context.project_id,
        variables: context.vars,
        context_manifest: prompt_context_manifest,
        temperature: 0.45,
        cancel_token: context.cancel_token,
        step: VideoPromptStepProgress::new(
            progress_span.first_half().start(),
            100,
            generation_message,
        )
        .with_span(progress_span.first_half()),
        progress,
        chunks,
        structured_progress: Some(video_prompt_progress_spec(context.is_chat_video)),
        checkpoint_context: "视频提示词初稿断点",
    })
    .await?;
    let validate = |value: &serde_json::Value| {
        parse_single_generated_video_prompt_shot(
            context.spec,
            &serde_json::to_string(value)?,
            context.segment_by_number,
            context.prompt_output_language,
            context.shot_number,
            context.allowed_reference_ids,
            context.spec.video_prompt_label,
        )
    };
    let (final_value, video_prompt, targeted_repair_used) = match validate(&raw.value) {
        Ok(video_prompt) => (raw.value.clone(), video_prompt, false),
        Err(validation_error) if !raw.restored => {
            let repaired = repair_invalid_video_prompt_fields(
                context,
                &raw.value,
                &validation_error,
                progress_span,
                progress,
                chunks,
            )
            .await?;
            let video_prompt = validate(&repaired).map_err(|repair_error| {
                AppError::InvalidArgument(format!(
                    "{}首次校验失败：{}；一次字段修复后仍不合法：{}",
                    context.spec.video_prompt_label, validation_error, repair_error
                ))
            })?;
            (repaired, video_prompt, true)
        }
        Err(error) => {
            return Err(AppError::InvalidState {
                current: format!("invalid_completed_video_prompt_checkpoint:{error}"),
                operation: format!("恢复{}调用级断点", context.spec.video_prompt_label),
            });
        }
    };
    if !raw.restored {
        save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
            service: context.service,
            workflow_run_id: draft_workflow_run_id,
            step_key: draft_step_key,
            step_order: call_order,
            input_fingerprint_json: draft_fingerprint_json,
            trace: &raw.trace,
            parsed_output: final_value.clone(),
            extra_json: json!({
                "structured_response": final_value,
            }),
            artifacts_json: Some(json!({
                "targeted_repair_used": targeted_repair_used,
                "full_object_repair_used": false,
            })),
        })
        .await?;
    }
    Ok(ExecutedVideoPromptUnit {
        video_prompt,
        generation_trace: video_prompt_generation_trace(raw.trace, context.workflow_run_id),
    })
}
