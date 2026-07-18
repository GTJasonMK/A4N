use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use a4n_prompt::rendering::PromptRenderer;

use super::super::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSourceContext,
    FramePromptChainInput, FramePromptGenerationSpec, OrchestratorContext, VideoFramePrompt,
    VideoSegmentPlan, VideoUnitPlan, ensure_video_reference_ids_known, frame_prompts_from_rows,
    validate_frame_prompts_for_video_prompt_inputs, video_units_for_segment_plan,
};

pub(super) fn select_boundary_video_prompt_inputs(
    frame_rows: Vec<chapter_video_prompt::Model>,
    segment_plan: &[VideoSegmentPlan],
    duration_secs: i32,
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
) -> Result<(Vec<VideoFramePrompt>, Vec<VideoUnitPlan>), AppError> {
    let actual_frame_count = frame_rows.len();
    let prompts = frame_prompts_from_rows(&frame_rows)?;
    let prompts = validate_frame_prompts_for_video_prompt_inputs(
        prompts,
        segment_plan,
        allowed_reference_ids,
        label,
    )?;
    let frame_by_number = frame_rows
        .iter()
        .zip(prompts.iter())
        .map(|(row, frame)| {
            if row.shot_number != frame.frame_number {
                return Err(AppError::InvalidArgument(format!(
                    "已保存{label} #{} 的 frame_number 必须等于存储编号，实际为 {}",
                    row.shot_number, frame.frame_number
                )));
            }
            Ok((frame.frame_number, frame.clone()))
        })
        .collect::<Result<HashMap<_, _>, AppError>>()?;

    let units = video_units_for_segment_plan(segment_plan, duration_secs)?;
    let mut selected_units = Vec::new();
    let mut selected_frame_numbers = BTreeSet::new();
    for unit in units {
        let Some(start_frame_number) = unit.start_frame_number else {
            continue;
        };
        let Some(end_frame_number) = unit.end_frame_number else {
            continue;
        };
        let Some(start_frame) = frame_by_number.get(&start_frame_number) else {
            continue;
        };
        let Some(end_frame) = frame_by_number.get(&end_frame_number) else {
            continue;
        };
        validate_boundary_frame_for_video_unit(
            start_frame,
            &unit.source_segment_numbers(),
            allowed_reference_ids,
            label,
        )?;
        validate_boundary_frame_for_video_unit(
            end_frame,
            &unit.source_segment_numbers(),
            allowed_reference_ids,
            label,
        )?;
        selected_units.push(unit);
        selected_frame_numbers.insert(start_frame_number);
        selected_frame_numbers.insert(end_frame_number);
    }

    if selected_units.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "当前章节{label}不足以生成视频提示词：需要至少一组连续边界帧，当前已有 {actual_frame_count} 条"
        )));
    }

    let selected_frames = prompts
        .into_iter()
        .filter(|frame| selected_frame_numbers.contains(&frame.frame_number))
        .collect::<Vec<_>>();
    Ok((selected_frames, selected_units))
}

fn validate_boundary_frame_for_video_unit(
    frame: &VideoFramePrompt,
    segment_numbers: &[i32],
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
) -> Result<(), AppError> {
    if !segment_numbers
        .iter()
        .any(|number| frame.segment_numbers.contains(number))
    {
        return Err(AppError::InvalidArgument(format!(
            "已保存{label} #{} 未声明当前视频单元的来源片段 {:?}，不能作为该单元视频提示词输入",
            frame.frame_number, segment_numbers
        )));
    }
    ensure_video_reference_ids_known(
        &frame.reference_image_ids,
        allowed_reference_ids,
        &format!("{label} #{}", frame.frame_number),
    )?;
    Ok(())
}

pub(super) struct ChapterVideoPromptChainInput<'a, 'source> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
    pub cancel_token: Option<&'a CancellationToken>,
}

pub(super) fn frame_prompt_chain_input<'a, 'source>(
    service: &'a OrchestratorContext,
    spec: &'a FramePromptGenerationSpec,
    input: ChapterVideoPromptStageInput<'a, 'source>,
) -> FramePromptChainInput<'a, 'source> {
    let ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode: _,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    } = input;
    FramePromptChainInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    }
}

pub(super) fn video_prompt_chain_input<'a, 'source>(
    service: &'a OrchestratorContext,
    spec: &'a FramePromptGenerationSpec,
    input: ChapterVideoPromptStageInput<'a, 'source>,
) -> ChapterVideoPromptChainInput<'a, 'source> {
    let ChapterVideoPromptStageInput {
        project_id,
        chapter_id,
        workflow_mode: _,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    } = input;
    ChapterVideoPromptChainInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        prompt_renderer,
        variables,
        options,
        cancel_token,
    }
}
