use std::collections::{HashMap, HashSet};

use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

use a4n_core::prompt_language::metadata_matches_prompt_output_language;

use super::super::{
    AppError, FramePromptGenerationSpec, GeneratedVideoPromptShot, NewChapterVideoPrompt,
    OrchestratorContext, PromptOutputLanguage, PromptWorkflowMode, VideoFramePrompt,
    VideoPromptGenerationTrace, VideoPromptProgress, VideoSegmentPlan, VideoUnitPlan,
    attach_generation_trace_to_draft, report_video_prompt_progress,
    validate_generated_video_prompts, validate_partial_generated_video_prompts,
    video_prompt_rows_for_workflow,
};
use super::contracts::{VideoPromptDraftsForSpecInput, video_prompt_drafts_for_spec};

pub(super) struct VideoPromptFinalizationInput<'a> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub source_metadata: &'a Value,
    pub segment_by_number: &'a HashMap<i32, &'a VideoSegmentPlan>,
    pub video_unit_by_number: &'a HashMap<i32, VideoUnitPlan>,
    pub frame_by_number: &'a HashMap<i32, VideoFramePrompt>,
    pub available_assets: &'a Value,
    pub generation_reference_ids: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
    pub execution_identity: &'a Value,
    pub video_prompts: &'a [GeneratedVideoPromptShot],
    pub context_video_prompts: &'a [GeneratedVideoPromptShot],
    pub preserved_generated_reused_shots: &'a HashSet<i32>,
    pub generated_video_prompt_traces: &'a HashMap<i32, VideoPromptGenerationTrace>,
    pub generated_this_run: usize,
    pub all_video_units: &'a [VideoUnitPlan],
    pub video_segment_plan: &'a [VideoSegmentPlan],
    pub regenerate_existing: bool,
}

pub(super) async fn finalize_video_prompt_chain(
    input: VideoPromptFinalizationInput<'_>,
    progress: &mut Option<VideoPromptProgress<'_>>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let VideoPromptFinalizationInput {
        service,
        spec,
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        segment_by_number,
        video_unit_by_number,
        frame_by_number,
        available_assets,
        generation_reference_ids,
        generation_asset_fingerprint,
        execution_identity,
        video_prompts,
        context_video_prompts,
        preserved_generated_reused_shots,
        generated_video_prompt_traces,
        generated_this_run,
        all_video_units,
        video_segment_plan,
        regenerate_existing,
    } = input;

    validate_partial_generated_video_prompts(
        spec,
        video_prompts,
        segment_by_number,
        prompt_output_language,
    )?;
    if video_prompts.len() == segment_by_number.len() {
        validate_generated_video_prompts(
            spec,
            video_prompts,
            segment_by_number,
            prompt_output_language,
        )?;
    }
    let video_drafts = video_prompt_drafts_for_spec(VideoPromptDraftsForSpecInput {
        spec,
        project_id,
        chapter_id,
        style,
        prompt_output_language,
        source_metadata,
        segment_by_number,
        video_unit_by_number,
        frame_by_number,
        available_assets,
        generation_reference_ids,
        generation_asset_fingerprint,
        execution_identity,
        video_prompts,
        preserved_generated_reused_shots,
    })?;
    let video_drafts =
        generated_video_prompt_traces
            .iter()
            .fold(video_drafts, |drafts, (shot_number, trace)| {
                attach_generation_trace_to_draft(
                    drafts,
                    spec.workflow_mode,
                    "video_prompt",
                    *shot_number,
                    trace.clone(),
                )
            });
    let rows = if video_drafts.is_empty() {
        video_prompt_result_rows(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            video_segment_plan,
            prompt_output_language,
        )
        .await?
    } else {
        sync_video_prompt_drafts(service, chapter_id, video_drafts, regenerate_existing).await?;
        video_prompt_result_rows(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            video_segment_plan,
            prompt_output_language,
        )
        .await?
    };
    let completed_shots = context_video_prompts
        .iter()
        .map(GeneratedVideoPromptShot::shot_number)
        .collect::<HashSet<_>>();
    let completed_count = completed_shots.len();
    let total_video_unit_count = all_video_units.len();
    if generated_this_run == 0 && completed_count < total_video_unit_count {
        return Err(no_new_video_prompt_progress_error(
            spec,
            all_video_units,
            &completed_shots,
            frame_by_number,
        ));
    }
    let message = if generated_this_run == 0 {
        format!(
            "{}已全部生成，共 {completed_count}/{} 条",
            spec.video_prompt_label, total_video_unit_count
        )
    } else {
        format!(
            "本批已生成 {} 条{}，当前共 {}/{} 条",
            generated_this_run, spec.video_prompt_label, completed_count, total_video_unit_count
        )
    };
    report_video_prompt_progress(progress, 100, 100, &message)?;
    Ok(rows)
}

fn no_new_video_prompt_progress_error(
    spec: &FramePromptGenerationSpec,
    all_video_units: &[VideoUnitPlan],
    completed_shots: &HashSet<i32>,
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
) -> AppError {
    let Some(next_unit) = all_video_units
        .iter()
        .find(|unit| !completed_shots.contains(&unit.unit_number))
    else {
        return AppError::InvalidArgument(format!(
            "{}未生成新内容；请刷新后重试",
            spec.video_prompt_label
        ));
    };
    let completed_count = completed_shots.len();
    let total_count = all_video_units.len();
    let unit_number = next_unit.unit_number;
    match spec.workflow_mode {
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo => {
            let start_frame = next_unit.start_frame_number;
            let end_frame = next_unit.end_frame_number;
            let mut missing = Vec::new();
            if start_frame.is_none_or(|frame| !frame_by_number.contains_key(&frame)) {
                let start_text =
                    start_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
                missing.push(format!("#{start_text}"));
            }
            if end_frame.is_none_or(|frame| !frame_by_number.contains_key(&frame)) {
                let end_text = end_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
                missing.push(format!("#{end_text}"));
            }
            let start_text = start_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
            let end_text = end_frame.map_or_else(|| "?".to_string(), |frame| frame.to_string());
            let missing_text = if missing.is_empty() {
                format!("视频单元 #{unit_number} 的连续边界帧")
            } else {
                format!("{} {}", spec.incomplete_frame_label, missing.join("、"))
            };
            AppError::InvalidArgument(format!(
                "当前章节{}不足，无法继续生成{}：已完成/可复用 {completed_count}/{total_count} 个视频单元；下一条视频提示词 #{} 需要连续边界帧 #{} 和 #{}，当前缺少{}。请先生成或补齐{}。",
                spec.incomplete_frame_label,
                spec.video_prompt_label,
                unit_number,
                start_text,
                end_text,
                missing_text,
                spec.incomplete_frame_label
            ))
        }
        PromptWorkflowMode::FirstFrameChain => AppError::InvalidArgument(format!(
            "当前章节{}不足，无法继续生成{}：已完成/可复用 {completed_count}/{total_count} 个视频单元；下一条视频提示词 #{} 缺少可承接的首帧输入。请先生成或补齐{}。",
            spec.incomplete_frame_label,
            spec.video_prompt_label,
            unit_number,
            spec.incomplete_frame_label
        )),
        PromptWorkflowMode::Storyboard => AppError::InvalidArgument(format!(
            "当前章节{}不足，无法继续生成{}：已完成/可复用 {completed_count}/{total_count} 个视频单元；下一条视频提示词 #{} 缺少同单元分镜提示词或分镜板。请先生成或补齐{}。",
            spec.incomplete_frame_label,
            spec.video_prompt_label,
            unit_number,
            spec.incomplete_frame_label
        )),
    }
}

async fn sync_video_prompt_drafts(
    service: &OrchestratorContext,
    chapter_id: i64,
    drafts: Vec<NewChapterVideoPrompt>,
    regenerate_existing: bool,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    if regenerate_existing {
        service.sync_chapter_prompts(chapter_id, drafts).await
    } else {
        service
            .sync_chapter_prompts_preserving_generated_video_prompts(chapter_id, drafts)
            .await
    }
}

async fn video_prompt_result_rows(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    segment_plan: &[VideoSegmentPlan],
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let segment_shots = segment_plan
        .iter()
        .map(|segment| segment.segment_number)
        .collect::<HashSet<_>>();
    Ok(
        video_prompt_rows_for_workflow(service, project_id, chapter_id, workflow_mode)
            .await?
            .into_iter()
            .filter(|row| segment_shots.contains(&row.shot_number))
            .filter(|row| {
                metadata_matches_prompt_output_language(
                    row.metadata_json.as_ref(),
                    prompt_output_language,
                )
            })
            .collect::<Vec<_>>(),
    )
}
