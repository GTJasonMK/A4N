use a4n_core::{error::AppError, source_spans::SourceSpan};
use a4n_llm::context::LlmContextManifest;
use serde_json::{Value, json};

use super::super::{
    CompletedVideoLlmCheckpointInput, ProgressSpan, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
    VideoContextManifestInput, VideoContextPreviousOutput, VideoPlanningPromptContext,
    VideoPromptRenderInput, VideoPromptStepProgress, build_segment_plan_for_shot_user,
    build_video_context_manifest, checkpoint_response_content, load_completed_video_checkpoint,
    parse_generated_segment_plan_with_current_config, parse_video_segment_plan_input_values,
    render_json_with_validation_with_context, report_video_prompt_checkpoint,
    report_video_prompt_progress, save_completed_video_llm_checkpoint, segment_plan_progress_spec,
    serialize_video_value, source_spans_for_shot,
    validate_segment_plan_against_shooting_script_projection, video_llm_call_trace_context,
    video_stage_fingerprint,
};
use super::{
    contracts::{RenderedSegmentPlanShots, StagedSegmentPlanInput},
    segment_plan_shot_recipe_artifacts,
};

struct PendingSegmentPlanShot {
    index: usize,
    source_shot_id: String,
    source_spans: Vec<SourceSpan>,
    context: String,
    context_manifest: LlmContextManifest,
    fingerprint_json: Value,
    expected_items: Option<usize>,
}

#[derive(Debug)]
struct ValidatedSegmentPlanShot {
    input_values: Vec<Value>,
    checkpoint_values: Vec<Value>,
}

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
        prompt_output_language,
        previous_segment_plan,
        previous_segment_recipes,
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
    let mut completed_shots = (0..total_shots)
        .map(|_| None)
        .collect::<Vec<Option<(ValidatedSegmentPlanShot, Value)>>>();
    let mut pending_shots = Vec::new();
    let mut restored_count = 0usize;
    let mut last_trace = None;
    for (index, shot) in shooting_script_projection.shots().enumerate() {
        if cancel_token.is_some_and(|token| token.is_cancelled()) {
            return Err(AppError::Cancelled);
        }
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
        let expected_items = Some(segment_plan_expected_items(
            shot.recommended_segment_count,
            shot.minimum_readable_segment_count,
        ));
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
                "validation_contract": "segment_plan_per_shot_v2",
            }),
        );
        if let Some(checkpoint) =
            load_completed_video_checkpoint(service, workflow_run_id, &step_key, &fingerprint_json)
                .await?
        {
            let (response_content, segment_shot) = restore_segment_plan_shot_checkpoint(
                &checkpoint,
                &shot_context,
                &shot_source_spans,
                shooting_script_projection,
                &shot.source_shot_id,
                duration_secs,
                default_aspect_ratio,
            )?;
            completed_shots[index] = Some((
                segment_shot,
                json!({
                    "source_shot_id": shot.source_shot_id,
                    "prompt_name": "chapter_video_segment_plan",
                    "raw": response_content
                }),
            ));
            restored_count += 1;
            continue;
        }

        pending_shots.push(PendingSegmentPlanShot {
            index,
            source_shot_id: shot.source_shot_id.clone(),
            source_spans: shot_source_spans,
            context: shot_context,
            context_manifest,
            fingerprint_json,
            expected_items,
        });
    }

    let pending_progress_span =
        pending_segment_plan_progress_span(progress_span, restored_count, total_shots);
    let restored_current = pending_progress_span.start();
    if restored_count > 0 {
        let restored_draft_text = completed_shots
            .iter()
            .enumerate()
            .filter_map(|(index, completed)| completed.as_ref().map(|shot| (index, shot)))
            .map(|(index, (segment_shot, _))| {
                segment_plan_draft_text(index + 1, &segment_shot.checkpoint_values)
            })
            .collect::<Result<Vec<_>, _>>()?
            .concat();
        let recovery_message = if pending_shots.is_empty() {
            format!("{progress_message}（已恢复全部 {restored_count} 个镜头断点）")
        } else {
            format!(
                "{progress_message}（已恢复 {restored_count} 个镜头断点，剩余 {} 个镜头继续生成或修复）",
                pending_shots.len()
            )
        };
        report_video_prompt_checkpoint(
            progress,
            restored_current,
            100,
            "video_segment_plan.recovered",
            restored_count,
            total_shots,
            restored_draft_text,
            recovery_message.clone(),
        )?;
        report_video_prompt_progress(progress, restored_current, 100, &recovery_message)?;
    }

    let pending_total = pending_shots.len();
    for (pending_index, pending) in pending_shots.into_iter().enumerate() {
        let PendingSegmentPlanShot {
            index,
            source_shot_id,
            source_spans: shot_source_spans,
            context: shot_context,
            context_manifest,
            fingerprint_json,
            expected_items,
        } = pending;
        let shot_span = pending_progress_span.split(pending_index, pending_total);
        let step_key = format!("video_segment_plan.shot.{source_shot_id}");
        let (trace, segment_shot, structured_response) = render_json_with_validation_with_context(
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
                        AppError::InvalidArgument("视频片段规划镜头序号超出 i32 范围".to_string())
                    })?,
            },
            |raw| {
                parse_and_validate_segment_plan_shot(
                    raw,
                    &shot_context,
                    &shot_source_spans,
                    shooting_script_projection,
                    &source_shot_id,
                    duration_secs,
                    default_aspect_ratio,
                )
            },
        )
        .await?;
        let stage_output = json!({
            "source_shot_id": source_shot_id.clone(),
            "prompt_name": trace.prompt_name.clone(),
            "raw": trace.response_text.clone()
        });
        let step_order = i32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(200))
            .ok_or_else(|| {
                AppError::InvalidArgument("视频片段规划镜头序号超出 i32 范围".to_string())
            })?;
        let checkpoint_key = step_key.clone();
        let previous_recipe_state = previous_segment_plan.zip(previous_segment_recipes);
        let recipe_artifacts = segment_plan_shot_recipe_artifacts(
            &segment_shot.checkpoint_values,
            style,
            prompt_output_language,
            previous_recipe_state,
        )?;
        save_completed_video_llm_checkpoint(CompletedVideoLlmCheckpointInput {
            service,
            workflow_run_id,
            step_key,
            step_order,
            input_fingerprint_json: fingerprint_json,
            trace: &trace,
            parsed_output: serde_json::to_value(&segment_shot.checkpoint_values)?,
            extra_json: json!({
                "stage_output": stage_output.clone(),
                "structured_response": structured_response,
            }),
            artifacts_json: Some(recipe_artifacts),
        })
        .await?;
        let draft_text = segment_plan_draft_text(index + 1, &segment_shot.checkpoint_values)?;
        report_video_prompt_checkpoint(
            progress,
            shot_span.end(),
            100,
            checkpoint_key,
            index + 1,
            total_shots,
            draft_text,
            format!("视频片段规划镜头 #{source_shot_id} 已保存"),
        )?;
        report_video_prompt_progress(
            progress,
            shot_span.end(),
            100,
            &format!("{progress_message}（镜头 #{source_shot_id} 已保存）"),
        )?;
        completed_shots[index] = Some((segment_shot, stage_output));
        last_trace = Some(trace);
    }

    if cancel_token.is_some_and(tokio_util::sync::CancellationToken::is_cancelled) {
        return Err(AppError::Cancelled);
    }
    report_video_prompt_progress(
        progress,
        progress_span.end().saturating_add(1).min(99),
        100,
        "所有镜头已保存，正在汇总校验片段规划",
    )?;

    let mut segment_inputs = Vec::<Value>::new();
    let mut stage_outputs = Vec::with_capacity(total_shots);
    for (index, completed) in completed_shots.into_iter().enumerate() {
        let Some((segment_shot, stage_output)) = completed else {
            return Err(AppError::InvalidState {
                current: "missing_segment_plan_shot".to_string(),
                operation: format!("汇总视频片段规划镜头 #{}", index + 1),
            });
        };
        segment_inputs.extend(segment_shot.input_values);
        stage_outputs.push(stage_output);
    }
    Ok(RenderedSegmentPlanShots {
        segment_inputs,
        stage_outputs,
        last_trace,
    })
}

fn restore_segment_plan_shot_checkpoint(
    checkpoint: &Value,
    context: &str,
    source_spans: &[a4n_core::source_spans::SourceSpan],
    shooting_script_projection: &super::super::VideoShootingScriptProjection,
    source_shot_id: &str,
    duration_secs: i32,
    default_aspect_ratio: &str,
) -> Result<(String, ValidatedSegmentPlanShot), AppError> {
    let response_content = checkpoint_response_content(checkpoint, context)?.to_string();
    let validation_content = checkpoint
        .get("structured_response")
        .map(serde_json::to_string)
        .transpose()?
        .unwrap_or_else(|| response_content.clone());
    let segment_shot = parse_and_validate_segment_plan_shot(
        &validation_content,
        context,
        source_spans,
        shooting_script_projection,
        source_shot_id,
        duration_secs,
        default_aspect_ratio,
    )?;
    ensure_segment_plan_checkpoint_output(checkpoint, &segment_shot, context)?;
    Ok((response_content, segment_shot))
}

fn parse_and_validate_segment_plan_shot(
    raw: &str,
    context: &str,
    source_spans: &[a4n_core::source_spans::SourceSpan],
    shooting_script_projection: &super::super::VideoShootingScriptProjection,
    source_shot_id: &str,
    duration_secs: i32,
    default_aspect_ratio: &str,
) -> Result<ValidatedSegmentPlanShot, AppError> {
    let input_values = parse_video_segment_plan_input_values(raw, context)?;
    let scoped_projection =
        shooting_script_projection_for_shot(shooting_script_projection, source_shot_id)?;
    let combined_raw = json!({ "segments": &input_values }).to_string();
    let (_, segment_plan) = parse_generated_segment_plan_with_current_config(
        &combined_raw,
        source_spans,
        &scoped_projection,
        duration_secs,
        default_aspect_ratio,
    )?;
    validate_segment_plan_against_shooting_script_projection(&segment_plan, &scoped_projection)?;
    let checkpoint_values = segment_plan
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ValidatedSegmentPlanShot {
        input_values,
        checkpoint_values,
    })
}

fn ensure_segment_plan_checkpoint_output(
    checkpoint: &Value,
    segment_shot: &ValidatedSegmentPlanShot,
    context: &str,
) -> Result<(), AppError> {
    let checkpoint_output =
        checkpoint
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_parsed_output".to_string(),
                operation: format!("读取{context}"),
            })?;
    if checkpoint_output == &serde_json::to_value(&segment_shot.checkpoint_values)?
        || checkpoint_output == &serde_json::to_value(&segment_shot.input_values)?
    {
        return Ok(());
    }
    Err(AppError::InvalidState {
        current: "checkpoint_parsed_output_mismatch".to_string(),
        operation: format!("校验{context}"),
    })
}

fn segment_plan_expected_items(recommended: i32, minimum_readable: i32) -> usize {
    recommended.max(minimum_readable).max(1) as usize
}

fn segment_plan_draft_text(
    shot_index: usize,
    segment_values: &[Value],
) -> Result<String, AppError> {
    Ok(format!(
        "{}\n",
        serde_json::to_string(&json!({
            "shot_index": shot_index,
            "segments": segment_values,
        }))?
    ))
}

fn pending_segment_plan_progress_span(
    progress_span: ProgressSpan,
    restored_count: usize,
    total_shots: usize,
) -> ProgressSpan {
    let restored_current = match restored_count.min(total_shots) {
        0 => progress_span.start(),
        count => progress_span.split(count - 1, total_shots).end(),
    };
    ProgressSpan::new(restored_current, progress_span.end())
}

fn shooting_script_projection_for_shot(
    shooting_script_projection: &super::super::VideoShootingScriptProjection,
    source_shot_id: &str,
) -> Result<super::super::VideoShootingScriptProjection, AppError> {
    let mut scoped = shooting_script_projection.clone();
    for scene in &mut scoped.scenes {
        scene
            .shots
            .retain(|shot| shot.source_shot_id == source_shot_id);
    }
    scoped.scenes.retain(|scene| !scene.shots.is_empty());
    if scoped.shot_count() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "视频片段规划引用不存在或不唯一的 source_shot_id {source_shot_id}"
        )));
    }
    Ok(scoped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::video_execution::{VideoExecutionProjectionV2, VideoShootingScriptIdentityV1};

    fn single_shot_projection(
        source_span_id: &str,
        duration_secs: i32,
        beat_count: usize,
        first_beat_seconds: Option<f64>,
    ) -> VideoExecutionProjectionV2 {
        let mut script = crate::video::tests::test_chapter_shooting_script(source_span_id);
        script.scenes[0].shots.truncate(1);
        script.scenes[0].shots[0]
            .performance_beats
            .truncate(beat_count);
        if let Some(seconds) = first_beat_seconds {
            script.scenes[0].shots[0].performance_beats[0].estimated_seconds = seconds;
        }
        super::super::super::video_projection_from_shooting_script(
            &script,
            VideoShootingScriptIdentityV1 {
                record_id: 1,
                revision: 1,
                source_revision_key: "test-revision".to_string(),
                source_content_sha256: "a".repeat(64),
                script_sha256: "b".repeat(64),
            },
            duration_secs,
        )
        .unwrap()
    }

    fn repeated_segment_plan_json(
        segment_count: usize,
        duration_secs: i32,
        covered_beat_ids: &[&str],
    ) -> String {
        let mut response: Value = serde_json::from_str(
            &crate::video::tests::segment_plan_json_for_shot(1, Some(duration_secs)),
        )
        .unwrap();
        let mut segment = response["segments"][0].clone();
        segment["covered_beat_ids"] = json!(covered_beat_ids);
        response["segments"] = json!(vec![segment; segment_count]);
        response.to_string()
    }

    #[test]
    fn per_shot_validation_rejects_output_below_minimum_readable_count() {
        let source_spans = a4n_core::source_spans::source_spans_for_content(1, 1, "章节正文");
        let projection = single_shot_projection(&source_spans[0].id, 2, 2, None);
        assert_eq!(
            projection
                .shot("shot-1")
                .unwrap()
                .minimum_readable_segment_count,
            2
        );
        assert_eq!(segment_plan_expected_items(1, 2), 2);

        let error = parse_and_validate_segment_plan_shot(
            &crate::video::tests::segment_plan_json_for_shot(1, Some(2)),
            "测试镜头",
            &source_spans,
            &projection,
            "shot-1",
            2,
            "16:9",
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("至少需要 2 个可读片段，实际只有 1 个")
        );
    }

    #[test]
    fn per_shot_checkpoint_contains_backend_derived_card_fields() {
        let source_spans = a4n_core::source_spans::source_spans_for_content(1, 1, "章节正文");
        let projection = single_shot_projection(&source_spans[0].id, 6, 1, None);
        let segment_shot = parse_and_validate_segment_plan_shot(
            &repeated_segment_plan_json(1, 6, &["beat-1"]),
            "测试镜头",
            &source_spans,
            &projection,
            "shot-1",
            6,
            "16:9",
        )
        .unwrap();

        assert_eq!(segment_shot.input_values[0].get("segment_number"), None);
        assert_eq!(segment_shot.input_values[0].get("duration_secs"), None);
        assert_eq!(segment_shot.input_values[0].get("aspect_ratio"), None);
        assert_eq!(segment_shot.checkpoint_values[0]["segment_number"], 1);
        assert_eq!(segment_shot.checkpoint_values[0]["duration_secs"], 6);
        assert_eq!(segment_shot.checkpoint_values[0]["aspect_ratio"], "16:9");
    }

    #[test]
    fn per_shot_validation_allows_one_long_beat_to_span_multiple_segments() {
        let source_spans = a4n_core::source_spans::source_spans_for_content(1, 1, "章节正文");
        let projection = single_shot_projection(&source_spans[0].id, 2, 1, Some(4.0));
        assert_eq!(
            projection
                .shot("shot-1")
                .unwrap()
                .minimum_readable_segment_count,
            2
        );
        let raw = repeated_segment_plan_json(2, 2, &["beat-1"]);

        let (_, segments) = parse_generated_segment_plan_with_current_config(
            &raw,
            &source_spans,
            &projection,
            2,
            "16:9",
        )
        .unwrap();
        validate_segment_plan_against_shooting_script_projection(&segments, &projection).unwrap();

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source_span_ids, segments[1].source_span_ids);
        assert_eq!(
            segments[0].source_span_ids,
            vec![source_spans[0].id.clone()]
        );
    }

    #[test]
    fn per_shot_validation_rejects_unnecessary_duplicate_beat_coverage() {
        let source_spans = a4n_core::source_spans::source_spans_for_content(1, 1, "章节正文");
        let projection = single_shot_projection(&source_spans[0].id, 2, 2, None);
        let raw = repeated_segment_plan_json(2, 2, &["beat-1"]);
        let (_, segments) = parse_generated_segment_plan_with_current_config(
            &raw,
            &source_spans,
            &projection,
            2,
            "16:9",
        )
        .unwrap();

        let error =
            validate_segment_plan_against_shooting_script_projection(&segments, &projection)
                .unwrap_err();

        assert!(error.to_string().contains(
            "必须且只能覆盖镜头 shot-1 的必需动作节拍 beat_id beat-1 一次，实际覆盖 2 次"
        ));
    }

    #[test]
    fn per_shot_validation_still_requires_every_required_beat_when_splitting() {
        let source_spans = a4n_core::source_spans::source_spans_for_content(1, 1, "章节正文");
        let projection = single_shot_projection(&source_spans[0].id, 1, 2, None);
        let raw = repeated_segment_plan_json(4, 1, &["beat-1"]);
        let (_, segments) = parse_generated_segment_plan_with_current_config(
            &raw,
            &source_spans,
            &projection,
            1,
            "16:9",
        )
        .unwrap();

        let error =
            validate_segment_plan_against_shooting_script_projection(&segments, &projection)
                .unwrap_err();

        assert!(
            error.to_string().contains(
                "必须至少覆盖镜头 shot-1 的必需动作节拍 beat_id beat-2 一次，实际覆盖 0 次"
            )
        );
    }

    #[test]
    fn pending_shots_use_restored_count_instead_of_checkpoint_position() {
        let progress_span = ProgressSpan::new(45, 90);

        let fresh = pending_segment_plan_progress_span(progress_span, 0, 100);
        assert_eq!((fresh.start(), fresh.end()), (45, 90));

        let mostly_restored = pending_segment_plan_progress_span(progress_span, 95, 100);
        assert_eq!((mostly_restored.start(), mostly_restored.end()), (87, 90));
        assert_eq!(mostly_restored.split(4, 5).end(), 90);

        let fully_restored = pending_segment_plan_progress_span(progress_span, 100, 100);
        assert_eq!((fully_restored.start(), fully_restored.end()), (90, 90));
    }
}
