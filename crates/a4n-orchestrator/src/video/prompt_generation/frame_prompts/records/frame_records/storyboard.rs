use std::collections::HashSet;

use a4n_core::error::AppError;

use crate::video::{
    library::ensure_video_reference_ids_known,
    prompt_json::{
        StoryboardPanel, VideoFramePrompt, VideoSegmentPlan, validate_reference_label_text,
    },
};

use super::super::super::{expected_video_unit_count, storyboard_expected_segment_numbers};

pub(in crate::video) fn validate_storyboard_prompts(
    storyboard_prompts: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
) -> Result<(), AppError> {
    let expected_count = expected_video_unit_count(segment_plan);
    if storyboard_prompts.len() != expected_count {
        return Err(AppError::InvalidArgument(format!(
            "分镜提示词数量必须等于视频执行单元数 N+B：当前 {} 条，内容片段数 {}，应为 {} 条",
            storyboard_prompts.len(),
            segment_plan.len(),
            expected_count
        )));
    }
    for (idx, frame) in storyboard_prompts.iter().enumerate() {
        let expected_number = one_based_storyboard_number(idx, "分镜提示词")?;
        if frame.frame_number != expected_number {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 frame_number 必须从 1 连续编号，位置 {} 应为 {}，实际为 {}",
                idx + 1,
                expected_number,
                frame.frame_number
            )));
        }
        let expected_segment_numbers =
            storyboard_expected_segment_numbers(segment_plan, frame.frame_number)?;
        let representative_segment_number =
            expected_segment_numbers.first().copied().ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "分镜提示词 #{} 缺少对应片段规划",
                    frame.frame_number
                ))
            })?;
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == representative_segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "分镜提示词 #{} 缺少对应片段规划",
                    frame.frame_number
                ))
            })?;
        if frame.segment_numbers != expected_segment_numbers {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 #{} segment_numbers 必须为 {:?}，实际为 {:?}",
                frame.frame_number, expected_segment_numbers, frame.segment_numbers
            )));
        }
        if frame.duration_secs < 1 {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 #{} 时长必须大于 0，实际为 {}s",
                frame.frame_number, frame.duration_secs
            )));
        }
        if frame.aspect_ratio != segment.aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词 #{} aspect_ratio 必须为 {}，实际为 {}",
                frame.frame_number, segment.aspect_ratio, frame.aspect_ratio
            )));
        }
        ensure_video_reference_ids_known(
            &frame.reference_image_ids,
            allowed_reference_ids,
            &format!("视频分镜 #{}", frame.frame_number),
        )?;
        validate_reference_label_text(
            &frame.frame_prompt,
            &format!("视频分镜 #{} frame_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        validate_reference_label_text(
            &frame.negative_prompt,
            &format!("视频分镜 #{} negative_prompt", frame.frame_number),
            frame.reference_image_ids.len(),
        )?;
        validate_storyboard_panels(frame)?;
    }
    Ok(())
}

fn validate_storyboard_panels(frame: &VideoFramePrompt) -> Result<(), AppError> {
    let panel_count = frame.storyboard_panels.len();
    if !(2..=6).contains(&panel_count) {
        return Err(AppError::InvalidArgument(format!(
            "视频分镜 #{} storyboard_panels 数量必须在 2 到 6 格之间，实际为 {}",
            frame.frame_number, panel_count
        )));
    }
    for (index, panel) in frame.storyboard_panels.iter().enumerate() {
        validate_storyboard_panel(frame.frame_number, index + 1, panel)?;
        if !frame_prompt_has_panel_marker(&frame.frame_prompt, index + 1) {
            return Err(AppError::InvalidArgument(format!(
                "视频分镜 #{} frame_prompt 必须逐格描述分镜板内容，缺少第 {} 格标记",
                frame.frame_number,
                index + 1
            )));
        }
    }
    Ok(())
}

fn validate_storyboard_panel(
    frame_number: i32,
    expected_index: usize,
    panel: &StoryboardPanel,
) -> Result<(), AppError> {
    let expected_panel_index = i32::try_from(expected_index).map_err(|_| {
        AppError::InvalidArgument(format!(
            "视频分镜 #{frame_number} storyboard panel 索引超过 i32 范围：{expected_index}"
        ))
    })?;
    if panel.panel_index != expected_panel_index {
        return Err(AppError::InvalidArgument(format!(
            "视频分镜 #{} storyboard_panels[{}].panel_index 必须为 {}，实际为 {}",
            frame_number,
            expected_index - 1,
            expected_index,
            panel.panel_index
        )));
    }
    for (field, value) in [
        ("moment", panel.moment.as_str()),
        ("visual_goal", panel.visual_goal.as_str()),
        ("composition", panel.composition.as_str()),
        ("continuity_note", panel.continuity_note.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频分镜 #{} storyboard_panels[{}].{} 不能为空",
                frame_number,
                expected_index - 1,
                field
            )));
        }
    }
    Ok(())
}

fn one_based_storyboard_number(index: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(index)
        .map_err(|_| AppError::InvalidArgument(format!("{label}索引超过 i32 范围：{index}")))?
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArgument(format!("{label}编号超过 i32 范围")))
}

fn frame_prompt_has_panel_marker(prompt: &str, panel_index: usize) -> bool {
    let compact = prompt.replace(' ', "");
    let lower = prompt.to_ascii_lowercase();
    compact.contains(&format!("第{panel_index}格"))
        || compact.contains(&format!("第{panel_index}个画格"))
        || lower.contains(&format!("panel {panel_index}"))
        || lower.contains(&format!("panel{panel_index}"))
}
