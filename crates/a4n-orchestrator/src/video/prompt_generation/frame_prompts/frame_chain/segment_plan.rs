use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::json;

use crate::video::ChapterVideoPromptStageInput;
use crate::video::prompt_generation::ProgressSpan;
use crate::video::prompt_generation::frame_prompts::{
    DownstreamSegmentPlanningGuardInput, PreparedVideoPromptInputs,
    ReusableSegmentPlanForSpecQuery, VideoPromptChunk, VideoPromptProgress,
    ensure_existing_segment_plans_can_regenerate,
    ensure_no_existing_downstream_prompts_before_segment_planning,
    ensure_no_existing_segment_plan_before_new_planning, ensure_segment_plan_generation_can_start,
    report_video_prompt_progress, reusable_segment_plan_for_spec,
};
use crate::video::prompt_store::{NewChapterVideoPromptInput, new_prompt_draft};

use super::contracts::{
    SegmentPlanForSpecInput, SegmentPlanFromShootingScriptProjectionInput,
    ShootingScriptProjectionPreparationInput,
};
use super::shooting_script_projection::render_segment_plan_from_shooting_script_projection;

pub(in crate::video::prompt_generation::frame_prompts) async fn generate_chapter_segment_plan_for_spec(
    input: SegmentPlanForSpecInput<'_, '_>,
    mut chunks: Option<VideoPromptChunk<'_>>,
    mut progress: Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let SegmentPlanForSpecInput {
        service,
        spec,
        stage,
    } = input;
    let ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode: _,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    } = stage;
    let PreparedVideoPromptInputs {
        source_metadata,
        source_spans,
        regenerate_existing,
        style,
        duration_secs,
        aspect_ratio,
        project_context,
        vars,
        workflow_run_id,
        ..
    } = service
        .prepare_video_prompt_inputs(project_id, chapter_id, &source, variables, options, true)
        .await?;

    let segment_storage_workflow_mode = spec.segment_storage_workflow_mode();
    let segment_storage_prompt_type = spec.segment_storage_prompt_type();
    let segment_storage_contract_version = spec.segment_storage_contract_version();
    let segment_storage_context = spec.segment_storage_context();
    let segment_storage_progress = spec.segment_storage_progress();
    if regenerate_existing {
        ensure_no_existing_downstream_prompts_before_segment_planning(
            DownstreamSegmentPlanningGuardInput {
                service,
                project_id,
                chapter_id,
                spec,
                source_metadata: &source_metadata,
                style: &style,
                aspect_ratio: &aspect_ratio,
                operation: "重新生成",
            },
        )
        .await?;
        ensure_existing_segment_plans_can_regenerate(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            &[],
        )
        .await?;
    } else {
        ensure_no_existing_downstream_prompts_before_segment_planning(
            DownstreamSegmentPlanningGuardInput {
                service,
                project_id,
                chapter_id,
                spec,
                source_metadata: &source_metadata,
                style: &style,
                aspect_ratio: &aspect_ratio,
                operation: "生成",
            },
        )
        .await?;
    }

    let reusable_segment = if regenerate_existing {
        None
    } else {
        reusable_segment_plan_for_spec(ReusableSegmentPlanForSpecQuery {
            service,
            project_id,
            chapter_id,
            spec,
            source_metadata: &source_metadata,
            source_spans: &source_spans,
            style: &style,
            duration_secs,
            aspect_ratio: &aspect_ratio,
        })
        .await?
    };
    if !regenerate_existing && reusable_segment.is_none() {
        ensure_no_existing_segment_plan_before_new_planning(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            segment_storage_context,
        )
        .await?;
        ensure_segment_plan_generation_can_start(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            &[],
        )
        .await?;
    }

    report_video_prompt_progress(&mut progress, 20, 100, segment_storage_progress)?;
    let (_segment_plan, segment_draft) = if let Some(existing) = reusable_segment {
        (existing.segment_plan, existing.draft)
    } else {
        let prepared = render_segment_plan_from_shooting_script_projection(
            SegmentPlanFromShootingScriptProjectionInput {
                shooting_script_projection: ShootingScriptProjectionPreparationInput {
                    service,
                    project_id,
                    chapter_id,
                    source,
                    source_spans: &source_spans,
                    project_context: project_context.as_ref(),
                    style: &style,
                    duration_secs,
                    aspect_ratio: &aspect_ratio,
                    prompt_renderer,
                    variables: &vars,
                    workflow_run_id,
                    progress_span: ProgressSpan::new(20, 45),
                    cancel_token,
                },
                segment_context: segment_storage_context,
                segment_progress: segment_storage_progress,
                segment_span: ProgressSpan::new(45, 90),
            },
            &mut progress,
            &mut chunks,
        )
        .await?;
        let canonical_segment_out = prepared.segment_json.to_string();
        let segment_draft = new_prompt_draft(NewChapterVideoPromptInput {
            project_id,
            chapter_id,
            workflow_mode: segment_storage_workflow_mode,
            style: Some(&style),
            prompt_type: segment_storage_prompt_type,
            shot_number: 0,
            title: Some(spec.segment_storage_title()),
            scene_summary: None,
            visual_prompt: &canonical_segment_out,
            motion_prompt: None,
            negative_prompt: None,
            duration_secs,
            aspect_ratio: Some(&aspect_ratio),
            reference_image_ids_json: Some(json!([])),
            metadata_json: Some(json!({
                "contract_version": segment_storage_contract_version,
                "video_execution_identity": prepared.execution_identity,
                "stage_outputs": prepared.segment_stage_outputs,
                "segments": prepared.segment_plan,
                "source": source_metadata.clone()
            })),
        })
        .with_generation_trace_if(prepared.segment_trace);
        (prepared.segment_plan, segment_draft)
    };
    let rows = service
        .sync_chapter_prompts(chapter_id, vec![segment_draft])
        .await?;
    report_video_prompt_progress(&mut progress, 100, 100, "视频片段规划记录已更新")?;
    Ok(rows)
}
