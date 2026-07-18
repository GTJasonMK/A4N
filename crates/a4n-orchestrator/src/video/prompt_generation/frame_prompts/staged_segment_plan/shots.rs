use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::{
    CompletedVideoLlmCheckpointInput, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
    VideoContextManifestInput, VideoContextPreviousOutput, VideoPlanningPromptContext,
    VideoPromptRenderInput, VideoPromptStepProgress, build_segment_plan_for_shot_user,
    build_video_context_manifest, checkpoint_response_content, ensure_checkpoint_parsed_output,
    load_completed_video_checkpoint, parse_video_segment_plan_input_values,
    render_json_with_validation_with_context, report_video_prompt_progress,
    save_completed_video_llm_checkpoint, segment_plan_progress_spec, serialize_video_value,
    source_spans_for_shot, video_llm_call_trace_context, video_stage_fingerprint,
};
use super::contracts::{RenderedSegmentPlanShots, StagedSegmentPlanInput};

pub(super) async fn render_segment_plan_shots(
    input: StagedSegmentPlanInput<'_, '_, '_, '_>,
    workflow_run_id: i64,
) -> Result<RenderedSegmentPlanShots, AppError> {
    let StagedSegmentPlanInput {
        service,
        prompt_renderer,
        project_id,
        workflow_run_id: _,
        variables,
        source,
        source_spans,
        project_context,
        shooting_script_projection_json,
        shooting_script_projection,
        default_aspect_ratio,
        style,
        duration_secs,
        context,
        progress_message,
        progress_span,
        cancel_token,
        progress,
        chunks,
    } = input;
    let total_shots = shooting_script_projection.shot_count();
    let planning_prompt_context = VideoPlanningPromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style,
        duration_secs,
        aspect_ratio: default_aspect_ratio,
    };
    let mut segment_inputs = Vec::<Value>::new();
    let mut stage_outputs = Vec::new();
    let mut last_trace = None;
    for (index, shot) in shooting_script_projection.shots().enumerate() {
        let shot_value = serialize_video_value(shot, "当前导演镜头")?;
        let shot_source_spans = source_spans_for_shot(source_spans, shot);
        if shot_source_spans.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频片段规划镜头 #{} 缺少可用于提示词的 source_spans",
                shot.source_shot_id
            )));
        }
        let user_content = build_segment_plan_for_shot_user(
            planning_prompt_context,
            &shot_value,
            &shot_source_spans,
        )
        .await?;
        let shot_context = format!("{context} 镜头 #{}", shot.source_shot_id);
        let shot_span = progress_span.split(index, total_shots);
        let expected_items = Some(shot.recommended_segment_count.max(1) as usize);
        let context_manifest = build_video_context_manifest(VideoContextManifestInput {
            prompt_renderer,
            project_id,
            variables,
            source: &source,
            source_spans,
            prompt_names: &["chapter_video_segment_plan"],
            previous_outputs: &[VideoContextPreviousOutput {
                pack_kind: VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
                source_name: "chapter_shooting_script_video_projection",
                parsed_output: shooting_script_projection_json,
                raw_output: None,
                placeholder: false,
            }],
            project_context,
            asset_context: None,
            dynamic_user_content: user_content.clone(),
        })
        .await?;
        let step_key = format!("video_segment_plan.shot.{}", shot.source_shot_id);
        let fingerprint_json = video_stage_fingerprint(
            "chapter_video_segment_plan",
            &context_manifest,
            json!({
                "project_id": project_id,
                "source_shot_id": shot.source_shot_id,
                "shot_index": index + 1,
                "total_shots": total_shots,
                "expected_items": expected_items,
            }),
        );
        if let Some(checkpoint) =
            load_completed_video_checkpoint(service, workflow_run_id, &step_key, &fingerprint_json)
                .await?
        {
            let response_content = checkpoint_response_content(&checkpoint, &shot_context)?;
            let parsed_output =
                checkpoint
                    .get("parsed_output")
                    .cloned()
                    .ok_or_else(|| AppError::InvalidState {
                        current: "missing_checkpoint_parsed_output".to_string(),
                        operation: format!("读取{shot_context}"),
                    })?;
            let segment_values = parse_video_segment_plan_input_values(
                &serde_json::to_string(&json!({"segments": parsed_output}))?,
                &shot_context,
            )?;
            let parsed_output = serde_json::to_value(&segment_values)?;
            ensure_checkpoint_parsed_output(&checkpoint, &parsed_output, &shot_context)?;
            stage_outputs.push(json!({
                "source_shot_id": shot.source_shot_id,
                "prompt_name": "chapter_video_segment_plan",
                "raw": response_content
            }));
            segment_inputs.extend(segment_values);
            report_video_prompt_progress(
                progress,
                shot_span.end(),
                100,
                &format!(
                    "{progress_message}（镜头 #{} 已复用断点）",
                    shot.source_shot_id
                ),
            )?;
            continue;
        }
        let (trace, segment_values, structured_response) =
            render_json_with_validation_with_context(
                VideoPromptRenderInput {
                    service,
                    prompt_renderer,
                    prompt_name: "chapter_video_segment_plan",
                    project_id,
                    variables,
                    context_manifest,
                    temperature: 0.35,
                    llm_call_trace: video_llm_call_trace_context(
                        workflow_run_id,
                        &step_key,
                        &fingerprint_json,
                    )?,
                    cancel_token,
                    step: VideoPromptStepProgress::new(shot_span.start(), 100, progress_message)
                        .with_span(shot_span),
                    progress,
                    chunks,
                    structured_progress: Some(segment_plan_progress_spec(expected_items)),
                },
                crate::structured_llm::StructuredRepairCheckpointContext {
                    workflow_run_id,
                    base_call_key: step_key.clone(),
                    call_order: i32::try_from(index)
                        .ok()
                        .and_then(|index| index.checked_add(200))
                        .ok_or_else(|| {
                            AppError::InvalidArgument(
                                "视频片段规划镜头序号超出 i32 范围".to_string(),
                            )
                        })?,
                },
                |raw| parse_video_segment_plan_input_values(raw, &shot_context),
            )
            .await?;
        let stage_output = json!({
            "source_shot_id": shot.source_shot_id,
            "prompt_name": trace.prompt_name.clone(),
            "raw": trace.response_text.clone()
        });
        let step_order = i32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(200))
            .ok_or_else(|| {
                AppError::InvalidArgument("视频片段规划镜头序号超出 i32 范围".to_string())
            })?;
        save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
            service,
            workflow_run_id,
            step_key,
            step_order,
            input_fingerprint_json: fingerprint_json,
            trace: &trace,
            parsed_output: serde_json::to_value(&segment_values)?,
            extra_json: json!({
                "stage_output": stage_output.clone(),
                "structured_response": structured_response,
            }),
            artifacts_json: None,
        })
        .await?;
        stage_outputs.push(stage_output);
        segment_inputs.extend(segment_values);
        last_trace = Some(trace);
    }
    Ok(RenderedSegmentPlanShots {
        segment_inputs,
        stage_outputs,
        last_trace,
    })
}
