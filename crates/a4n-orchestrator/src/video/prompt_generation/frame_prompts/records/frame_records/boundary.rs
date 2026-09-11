use std::collections::HashSet;

use a4n_error::AppError;

use crate::video::{
    library::ensure_video_reference_ids_known,
    prompt_json::{VideoFramePrompt, VideoSegmentPlan, validate_reference_label_text},
};

use super::super::super::{
    boundary_frame_expected_segment_numbers_from_units, expected_boundary_frame_count,
    units_for_frame_mapping,
};

pub(in crate::video) fn validate_frame_prompts(
    frame_prompts: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
) -> Result<(), AppError> {
    let expected_frame_count = expected_boundary_frame_count(segment_plan);
    if frame_prompts.len() != expected_frame_count {
        return Err(AppError::InvalidArgument(format!(
            "视频边界帧提示词数量必须等于执行单元数 U + 1：当前 {} 条，内容片段数 {}，应为 {} 条",
            frame_prompts.len(),
            segment_plan.len(),
            expected_frame_count
        )));
    }
    let mut units = None;
    let mut known_segment_numbers = None;
    for (idx, frame) in frame_prompts.iter().enumerate() {
        let expected_frame_number = i32::try_from(idx)
            .map_err(|_| {
                AppError::InvalidArgument(format!("视频首尾帧提示词索引超过 i32 范围：{idx}"))
            })?
            .checked_add(1)
            .ok_or_else(|| {
                AppError::InvalidArgument("视频首尾帧提示词编号超过 i32 范围".to_string())
            })?;
        if frame.frame_number != expected_frame_number {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number 必须从 1 连续编号，位置 {} 应为 {}，实际为 {}",
                idx + 1,
                expected_frame_number,
                frame.frame_number
            )));
        }
        if units.is_none() {
            units = Some(units_for_frame_mapping(segment_plan)?);
            known_segment_numbers = Some(
                segment_plan
                    .iter()
                    .map(|segment| segment.segment_number)
                    .collect::<HashSet<_>>(),
            );
        }
        let units = units.as_ref().expect("boundary frame units initialized");
        let known_segment_numbers = known_segment_numbers
            .as_ref()
            .expect("boundary frame segment index initialized");
        let expected_segment_numbers =
            boundary_frame_expected_segment_numbers_from_units(units, frame.frame_number)?;
        if frame.segment_numbers != expected_segment_numbers {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number {} segment_numbers 必须为 {:?}，实际为 {:?}",
                frame.frame_number, expected_segment_numbers, frame.segment_numbers
            )));
        }
        if frame.duration_secs < 1 {
            return Err(AppError::InvalidArgument(format!(
                "视频首尾帧提示词 frame_number {} 时长必须大于 0，实际为 {}s",
                frame.frame_number, frame.duration_secs
            )));
        }
        for segment_number in &frame.segment_numbers {
            if !known_segment_numbers.contains(segment_number) {
                return Err(AppError::InvalidArgument(format!(
                    "视频首尾帧提示词 frame_number {} 引用了不存在的片段 {}",
                    frame.frame_number, segment_number
                )));
            }
        }
        ensure_video_reference_ids_known(
            &frame.reference_image_ids,
            allowed_reference_ids,
            &format!("视频边界帧 #{}", frame.frame_number),
        )?;
        validate_reference_label_text(
            &frame.frame_prompt,
            &format!("视频边界帧 #{} frame_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        validate_reference_label_text(
            &frame.negative_prompt,
            &format!("视频边界帧 #{} negative_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        ensure_no_storyboard_panels(frame, &format!("视频边界帧 #{}", frame.frame_number))?;
    }
    Ok(())
}

pub(super) fn ensure_no_storyboard_panels(
    frame: &VideoFramePrompt,
    label: &str,
) -> Result<(), AppError> {
    if !frame.storyboard_panels.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{label} 不能输出 storyboard_panels；只有分镜模式的分镜板提示词可以输出多格分镜结构"
        )));
    }
    Ok(())
}
