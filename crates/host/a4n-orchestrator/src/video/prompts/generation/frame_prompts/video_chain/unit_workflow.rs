use std::collections::{BTreeMap, HashMap, HashSet};

use a4n_domain::prompt_language::PromptOutputLanguage;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use a4n_prompt::rendering::PromptRenderer;

use super::super::{
    AppError, ChapterVideoSourceContext, FramePromptGenerationSpec, GeneratedVideoPromptShot,
    OrchestratorContext, ProgressSpan, VideoPromptChunk, VideoPromptGenerationTrace,
    VideoPromptProgress, VideoSegmentPlan, report_video_prompt_progress,
};
use super::unit_execution::{VideoPromptUnitExecutionInput, execute_video_prompt_unit};

pub(super) struct VideoPromptUnitWorkflowInput<'a, 'source> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: &'a ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [a4n_domain::source_spans::SourceSpan],
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub vars: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub video_segment_plan: &'a [VideoSegmentPlan],
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_segment_json: &'a Value,
    pub frame_json: &'a Value,
    pub available_assets: &'a Value,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub is_chat_video: bool,
    pub generation_limit: usize,
    pub reusable_video_prompts: &'a BTreeMap<i32, GeneratedVideoPromptShot>,
    pub existing_generated_video_prompts: &'a BTreeMap<i32, GeneratedVideoPromptShot>,
    pub matching_generated_video_shots: &'a HashSet<i32>,
    pub regenerate_existing: bool,
    pub target_shot_number: Option<i32>,
}

pub(super) struct GeneratedVideoPromptUnits {
    pub video_prompts: Vec<GeneratedVideoPromptShot>,
    pub context_video_prompts: Vec<GeneratedVideoPromptShot>,
    pub preserved_generated_reused_shots: HashSet<i32>,
    pub generated_video_prompt_traces: HashMap<i32, VideoPromptGenerationTrace>,
    pub generated_this_run: usize,
}

pub(super) async fn generate_video_prompt_units(
    input: VideoPromptUnitWorkflowInput<'_, '_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
    chunks: &mut Option<VideoPromptChunk<'_>>,
) -> Result<GeneratedVideoPromptUnits, AppError> {
    let VideoPromptUnitWorkflowInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        prompt_renderer,
        vars,
        workflow_run_id,
        cancel_token,
        video_segment_plan,
        segment_by_number,
        video_segment_json,
        frame_json,
        available_assets,
        style,
        prompt_output_language,
        duration_secs,
        aspect_ratio,
        reference_ids,
        allowed_reference_ids,
        is_chat_video,
        generation_limit,
        reusable_video_prompts,
        existing_generated_video_prompts,
        matching_generated_video_shots,
        regenerate_existing,
        target_shot_number,
    } = input;
    let mut video_prompts = Vec::with_capacity(video_segment_plan.len());
    let mut context_video_prompts = Vec::with_capacity(video_segment_plan.len());
    let mut preserved_generated_reused_shots = HashSet::new();
    let mut generated_video_prompt_traces = HashMap::new();
    let mut generated_this_run = 0usize;
    let mut reuse_prefix = true;

    report_video_prompt_progress(progress, 60, 100, spec.generate_video_progress)?;
    for (index, segment) in video_segment_plan.iter().enumerate() {
        let current_index = index + 1;
        let total_segments = video_segment_plan.len();
        let shot_number = segment.segment_number;

        if target_shot_number.is_some_and(|target| target != shot_number) {
            if let Some(reusable) = existing_generated_video_prompts
                .get(&shot_number)
                .or_else(|| reusable_video_prompts.get(&shot_number))
            {
                video_prompts.push(reusable.clone());
                context_video_prompts.push(reusable.clone());
            }
            continue;
        }

        if spec.is_first_frame_chain() && generated_this_run >= generation_limit {
            if let Some(reusable) = reusable_video_prompts.get(&shot_number) {
                video_prompts.push(reusable.clone());
                context_video_prompts.push(reusable.clone());
                report_video_prompt_progress(
                    progress,
                    ProgressSpan::new(60, 90).split(index, total_segments).end(),
                    100,
                    &format!(
                        "已保留{}全章第 {current_index}/{total_segments} 段，本批已达到 {generation_limit} 条",
                        spec.video_prompt_label
                    ),
                )?;
                continue;
            }
            break;
        }
        if !regenerate_existing
            && let Some(existing_generated) = existing_generated_video_prompts.get(&shot_number)
        {
            preserved_generated_reused_shots.insert(shot_number);
            video_prompts.push(existing_generated.clone());
            context_video_prompts.push(existing_generated.clone());
            report_video_prompt_progress(
                progress,
                ProgressSpan::new(60, 90).split(index, total_segments).end(),
                100,
                &format!(
                    "已保留已生成视频的{}全章第 {current_index}/{total_segments} 段，继续生成后续段",
                    spec.video_prompt_label
                ),
            )?;
            continue;
        }
        if let Some(reusable) = reusable_video_prompts.get(&shot_number) {
            let can_reuse = if spec.is_first_frame_chain() {
                reuse_prefix && (!regenerate_existing || generated_this_run >= generation_limit)
            } else {
                !regenerate_existing || generated_this_run >= generation_limit
            };
            if can_reuse {
                if matching_generated_video_shots.contains(&shot_number) {
                    preserved_generated_reused_shots.insert(shot_number);
                }
                video_prompts.push(reusable.clone());
                context_video_prompts.push(reusable.clone());
                let reuse_message = if regenerate_existing {
                    format!(
                        "已保留{}全章第 {current_index}/{total_segments} 段，本批已达到 {generation_limit} 条",
                        spec.video_prompt_label
                    )
                } else {
                    format!(
                        "已复用{}全章第 {current_index}/{total_segments} 段",
                        spec.video_prompt_label
                    )
                };
                report_video_prompt_progress(
                    progress,
                    ProgressSpan::new(60, 90).split(index, total_segments).end(),
                    100,
                    &reuse_message,
                )?;
                continue;
            }
        }
        if spec.is_first_frame_chain() {
            reuse_prefix = false;
        }
        if generated_this_run >= generation_limit {
            continue;
        }

        let selected_spans = super::super::checkpoint::source_spans_for_segments(
            source_spans,
            std::iter::once(segment),
        )?;
        let source_text =
            a4n_domain::source_spans::source_span_context_excerpt(source.content, &selected_spans)
                .map_err(AppError::InvalidArgument)?;
        let scoped_source = ChapterVideoSourceContext {
            content: &source_text,
            summary: source.summary,
            analysis: source.analysis,
        };
        let executed = execute_video_prompt_unit(
            VideoPromptUnitExecutionInput {
                service,
                spec,
                project_id,
                chapter_id,
                source: &scoped_source,
                prompt_renderer,
                vars,
                workflow_run_id,
                cancel_token,
                segment,
                segment_by_number,
                video_segment_json,
                frame_json,
                previous_completed_shot: context_video_prompts.last(),
                available_assets,
                style,
                prompt_output_language,
                duration_secs,
                aspect_ratio,
                reference_ids,
                allowed_reference_ids,
                is_chat_video,
                segment_index: index,
                current_index,
                total_segments,
                generation_number: generated_this_run + 1,
                generation_limit,
            },
            progress,
            chunks,
        )
        .await?;
        generated_video_prompt_traces.insert(shot_number, executed.generation_trace);
        context_video_prompts.push(executed.video_prompt.clone());
        video_prompts.push(executed.video_prompt);
        generated_this_run += 1;
    }

    Ok(GeneratedVideoPromptUnits {
        video_prompts,
        context_video_prompts,
        preserved_generated_reused_shots,
        generated_video_prompt_traces,
        generated_this_run,
    })
}
