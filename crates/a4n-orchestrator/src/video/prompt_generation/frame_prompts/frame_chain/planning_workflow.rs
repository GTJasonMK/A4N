use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::video::ChapterVideoSourceContext;
use crate::video::prompt_generation::ProgressSpan;
use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, OrchestratorContext,
    ReusableSegmentPlanForSpecQuery, VideoPromptChunk, VideoPromptProgress,
    ensure_existing_frame_prompts_can_regenerate, ensure_existing_segment_plans_can_regenerate,
    ensure_no_existing_segment_plan_before_new_planning, ensure_segment_plan_generation_can_start,
    report_video_prompt_progress, reusable_segment_plan_for_spec,
};
use crate::video::prompt_json::VideoSegmentPlan;
use crate::video::prompt_store::{NewChapterVideoPromptInput, new_prompt_draft};
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::contracts::{
    SegmentPlanFromShootingScriptProjectionInput, ShootingScriptProjectionPreparationInput,
};
use super::shooting_script_projection::render_segment_plan_from_shooting_script_projection;

pub(super) struct PlanningWorkflowInput<'a, 'source, 'progress, 'chunk> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub project_context: Option<&'a Value>,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub variables: &'a HashMap<String, Value>,
    pub regenerate_existing: bool,
    pub workflow_run_id: Option<i64>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) struct PreparedPlanningWorkflow {
    pub segment_json: Value,
    pub segment_plan: Vec<VideoSegmentPlan>,
    pub segment_draft: Option<NewChapterVideoPrompt>,
    pub execution_identity: Value,
}

pub(super) async fn prepare_frame_chain_planning(
    input: PlanningWorkflowInput<'_, '_, '_, '_>,
) -> Result<PreparedPlanningWorkflow, AppError> {
    let PlanningWorkflowInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_metadata,
        source_spans,
        project_context,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        variables: vars,
        regenerate_existing,
        workflow_run_id,
        prompt_renderer,
        cancel_token,
        mut progress,
        mut chunks,
    } = input;
    let segment_storage_workflow_mode = spec.segment_storage_workflow_mode();
    let segment_storage_prompt_type = spec.segment_storage_prompt_type();
    let segment_storage_contract_version = spec.segment_storage_contract_version();
    let segment_storage_context = spec.segment_storage_context();
    let segment_storage_progress = spec.segment_storage_progress();
    if regenerate_existing {
        ensure_existing_frame_prompts_can_regenerate(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            spec.frame_prompt_type,
            prompt_output_language,
        )
        .await?;
        ensure_existing_segment_plans_can_regenerate(
            service,
            project_id,
            chapter_id,
            segment_storage_workflow_mode,
            segment_storage_prompt_type,
            spec.downstream_frame_prompt_types,
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
            spec.downstream_frame_prompt_types,
        )
        .await?;
    }
    report_video_prompt_progress(&mut progress, 20, 100, segment_storage_progress)?;
    let (segment_json, segment_plan, segment_draft, execution_identity) =
        if let Some(existing) = reusable_segment {
            (
                existing.segment_json,
                existing.segment_plan,
                None,
                existing.execution_identity,
            )
        } else {
            let prepared = render_segment_plan_from_shooting_script_projection(
                SegmentPlanFromShootingScriptProjectionInput {
                    shooting_script_projection: ShootingScriptProjectionPreparationInput {
                        service,
                        project_id,
                        chapter_id,
                        source,
                        source_spans: &source_spans,
                        project_context,
                        style: &style,
                        duration_secs,
                        aspect_ratio: &aspect_ratio,
                        prompt_renderer,
                        variables: &vars,
                        workflow_run_id,
                        progress_span: ProgressSpan::new(25, 45),
                        cancel_token,
                    },
                    segment_context: segment_storage_context,
                    segment_progress: segment_storage_progress,
                    segment_span: ProgressSpan::new(45, 60),
                },
                &mut progress,
                &mut chunks,
            )
            .await?;
            let segment_stage_outputs = prepared.segment_stage_outputs;
            let segment_trace = prepared.segment_trace;
            let execution_identity = prepared.execution_identity;
            let segment_json = prepared.segment_json;
            let canonical_segment_out = segment_json.to_string();
            let segment_plan = prepared.segment_plan;
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
                    "video_execution_identity": execution_identity,
                    "stage_outputs": segment_stage_outputs,
                    "segments": segment_plan,
                    "source": source_metadata.clone()
                })),
            })
            .with_generation_trace_if(segment_trace);
            (
                segment_json,
                segment_plan,
                Some(segment_draft),
                execution_identity,
            )
        };

    Ok(PreparedPlanningWorkflow {
        segment_json,
        segment_plan,
        segment_draft,
        execution_identity,
    })
}
