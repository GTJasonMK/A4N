use std::collections::HashMap;

use a4n_core::error::AppError;

use crate::video::prompt_json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::prompt_store::PromptWorkflowMode;

use super::super::VideoUnitPlan;

pub(in crate::video) fn video_prompt_segments_with_reference_ids(
    workflow_mode: PromptWorkflowMode,
    segments: &[VideoSegmentPlan],
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Vec<VideoSegmentPlan> {
    segments
        .iter()
        .map(|segment| {
            let mut segment = segment.clone();
            segment.reference_image_ids = video_prompt_reference_ids_for_segment(
                workflow_mode,
                segment.segment_number,
                frame_by_number,
                frame_image_ids_by_number,
                generation_reference_ids,
            );
            segment
        })
        .collect()
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_prompt_unit_segments_with_reference_ids(
    workflow_mode: PromptWorkflowMode,
    units: &[VideoUnitPlan],
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    units
        .iter()
        .map(|unit| {
            let mut segment = video_unit_as_prompt_segment(unit)?;
            segment.reference_image_ids = video_prompt_reference_ids_for_unit(
                workflow_mode,
                unit,
                frame_by_number,
                frame_image_ids_by_number,
                generation_reference_ids,
            );
            Ok(segment)
        })
        .collect()
}

fn video_unit_as_prompt_segment(unit: &VideoUnitPlan) -> Result<VideoSegmentPlan, AppError> {
    let mut segment = unit.prompt_segment.clone();
    retime_segment_performance_time_blocks(&mut segment, unit.duration_secs)?;
    segment.duration_secs = unit.duration_secs;
    Ok(segment)
}

fn retime_segment_performance_time_blocks(
    segment: &mut VideoSegmentPlan,
    duration_secs: i32,
) -> Result<(), AppError> {
    let blocks = &mut segment.performance_plan.time_blocks;
    if blocks.is_empty() {
        return Ok(());
    }

    let source_end_sec = blocks
        .last()
        .map(|block| block.end_sec)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频单元 #{} 无法从 performance_plan.time_blocks 推导原始时长",
                segment.segment_number
            ))
        })?;
    let scale = f64::from(duration_secs) / source_end_sec;
    let last_index = blocks.len() - 1;
    let mut next_start_sec = 0.0_f64;

    for (index, block) in blocks.iter_mut().enumerate() {
        let field = format!(
            "视频单元 #{} time_blocks[{}]",
            segment.segment_number,
            index + 1
        );
        let source_end = block.end_sec;
        if !source_end.is_finite() {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 必须是有限数字"
            )));
        }
        let end_sec = if index == last_index {
            f64::from(duration_secs)
        } else {
            (source_end * scale * 100.0).round() / 100.0
        };
        if end_sec <= next_start_sec {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 重定时后必须大于 start_sec"
            )));
        }
        block.start_sec = next_start_sec;
        block.end_sec = end_sec;
        next_start_sec = end_sec;
    }
    Ok(())
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_prompt_reference_ids_for_unit(
    workflow_mode: PromptWorkflowMode,
    unit: &VideoUnitPlan,
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Vec<i64> {
    let mut refs = Vec::new();
    match workflow_mode {
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo => {
            for frame_number in [unit.start_frame_number, unit.end_frame_number]
                .into_iter()
                .flatten()
            {
                if let Some(image_id) = frame_image_ids_by_number.get(&frame_number) {
                    extend_unique_reference_ids(&mut refs, &[*image_id]);
                }
                if let Some(frame) = frame_by_number.get(&frame_number) {
                    extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
                }
            }
        }
        PromptWorkflowMode::FirstFrameChain => {
            if unit.unit_number == 1
                && let Some(image_id) = frame_image_ids_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if unit.unit_number == 1
                && let Some(frame) = frame_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
        PromptWorkflowMode::Storyboard => {
            if let Some(storyboard_number) = unit.storyboard_number {
                if let Some(image_id) = frame_image_ids_by_number.get(&storyboard_number) {
                    extend_unique_reference_ids(&mut refs, &[*image_id]);
                }
                if let Some(frame) = frame_by_number.get(&storyboard_number) {
                    extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
                }
            }
        }
    }
    extend_unique_reference_ids(&mut refs, generation_reference_ids);
    refs
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_prompt_reference_ids_for_segment(
    workflow_mode: PromptWorkflowMode,
    segment_number: i32,
    frame_by_number: &HashMap<i32, VideoFramePrompt>,
    frame_image_ids_by_number: &HashMap<i32, i64>,
    generation_reference_ids: &[i64],
) -> Vec<i64> {
    let mut refs = Vec::new();
    match workflow_mode {
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo => {
            if let Some(image_id) = frame_image_ids_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if let Some(image_id) = frame_image_ids_by_number.get(&(segment_number + 1)) {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if let Some(frame) = frame_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
            if let Some(frame) = frame_by_number.get(&(segment_number + 1)) {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
        PromptWorkflowMode::FirstFrameChain => {
            if segment_number == 1
                && let Some(image_id) = frame_image_ids_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if segment_number == 1
                && let Some(frame) = frame_by_number.get(&1)
            {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
        PromptWorkflowMode::Storyboard => {
            if let Some(image_id) = frame_image_ids_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &[*image_id]);
            }
            if let Some(frame) = frame_by_number.get(&segment_number) {
                extend_unique_reference_ids(&mut refs, &frame.reference_image_ids);
            }
        }
    }
    extend_unique_reference_ids(&mut refs, generation_reference_ids);
    refs
}

pub(in crate::video::prompt_generation::frame_prompts) fn extend_unique_reference_ids(
    target: &mut Vec<i64>,
    source: &[i64],
) {
    for image_id in source {
        if *image_id > 0 && !target.contains(image_id) {
            target.push(*image_id);
        }
    }
}

pub(in crate::video::prompt_generation::frame_prompts) fn video_prompt_workflow_reference_ids(
    segments: &[VideoSegmentPlan],
) -> Vec<i64> {
    let mut refs = Vec::new();
    for segment in segments {
        extend_unique_reference_ids(&mut refs, &segment.reference_image_ids);
    }
    refs
}
