use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;

use super::{
    VideoFramePrompt, VideoSegmentPlan, boundary_frame_expected_segment_numbers,
    boundary_frame_expected_segment_numbers_from_units, ensure_video_reference_ids_known,
    frame_prompts_from_rows, units_for_frame_mapping, validate_direct_prompt_text,
    validate_reference_label_text,
};

pub(super) fn validate_single_boundary_frame_prompt(
    frame: &VideoFramePrompt,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    required_first_reference_image_id: Option<i64>,
) -> Result<(), AppError> {
    let expected_segment_numbers =
        boundary_frame_expected_segment_numbers(segment_plan, frame.frame_number)?;
    let segment_by_number = segment_index(segment_plan);
    validate_single_boundary_frame_prompt_with_context(
        frame,
        &expected_segment_numbers,
        &segment_by_number,
        allowed_reference_ids,
        required_first_reference_image_id,
    )
}

fn validate_single_boundary_frame_prompt_with_context(
    frame: &VideoFramePrompt,
    expected_segment_numbers: &[i32],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    allowed_reference_ids: &HashSet<i64>,
    required_first_reference_image_id: Option<i64>,
) -> Result<(), AppError> {
    if frame.segment_numbers != expected_segment_numbers {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 #{} segment_numbers 必须为 {:?}，实际为 {:?}",
            frame.frame_number, expected_segment_numbers, frame.segment_numbers
        )));
    }
    let expected_duration =
        boundary_frame_expected_duration_from_index(&frame.segment_numbers, segment_by_number)?;
    if frame.duration_secs != expected_duration {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 #{} duration_secs 必须为 {}，实际为 {}",
            frame.frame_number, expected_duration, frame.duration_secs
        )));
    }
    for segment_number in &frame.segment_numbers {
        let segment = segment_by_number
            .get(segment_number)
            .copied()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "边界帧 #{} 引用了不存在的片段 {}",
                    frame.frame_number, segment_number
                ))
            })?;
        if frame.aspect_ratio != segment.aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "边界帧 #{} aspect_ratio 必须为 {}，实际为 {}",
                frame.frame_number, segment.aspect_ratio, frame.aspect_ratio
            )));
        }
    }
    validate_direct_prompt_text(
        &frame.frame_prompt,
        &format!("边界帧 #{} frame_prompt", frame.frame_number),
    )?;
    validate_direct_prompt_text(
        &frame.negative_prompt,
        &format!("边界帧 #{} negative_prompt", frame.frame_number),
    )?;
    validate_reference_label_text(
        &frame.frame_prompt,
        &format!("边界帧 #{} frame_prompt", frame.frame_number),
        frame.reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &frame.negative_prompt,
        &format!("边界帧 #{} negative_prompt", frame.frame_number),
        frame.reference_image_ids.len(),
    )?;
    ensure_video_reference_ids_known(
        &frame.reference_image_ids,
        allowed_reference_ids,
        &format!("视频边界帧 #{}", frame.frame_number),
    )?;
    if let Some(required_id) = required_first_reference_image_id
        && frame.reference_image_ids.first().copied() != Some(required_id)
    {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 #{} reference_image_ids[0] 必须是上一帧实际图片 ID {}，这样 frame_prompt 中的“参考图 1”才对应上一帧；实际为 {:?}",
            frame.frame_number,
            required_id,
            frame.reference_image_ids.first()
        )));
    }
    Ok(())
}

pub(super) fn validate_partial_boundary_frame_prompts(
    frames: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<(), AppError> {
    let mut units = None;
    let mut segment_by_number = None;
    for (index, frame) in frames.iter().enumerate() {
        let expected_frame_number = one_based_frame_number(index, "边界帧提示词")?;
        if frame.frame_number != expected_frame_number {
            return Err(AppError::InvalidArgument(format!(
                "边界帧提示词必须是从 1 开始的连续前缀，位置 {} 应为 #{}，实际为 #{}",
                index + 1,
                expected_frame_number,
                frame.frame_number
            )));
        }
        if units.is_none() {
            units = Some(units_for_frame_mapping(segment_plan)?);
            segment_by_number = Some(segment_index(segment_plan));
        }
        let expected_segment_numbers = boundary_frame_expected_segment_numbers_from_units(
            units.as_ref().expect("boundary frame units initialized"),
            frame.frame_number,
        )?;
        validate_single_boundary_frame_prompt_with_context(
            frame,
            &expected_segment_numbers,
            segment_by_number
                .as_ref()
                .expect("boundary frame segment index initialized"),
            allowed_reference_ids,
            frame
                .frame_number
                .checked_sub(1)
                .and_then(|number| image_id_by_frame_number.get(&number).copied()),
        )?;
    }
    Ok(())
}

pub(in crate::video) fn boundary_frame_prompt_rows_prefix(
    rows: Vec<chapter_video_prompt::Model>,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    image_id_by_frame_number: &HashMap<i32, i64>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let mut prefix = Vec::new();
    let mut units = None;
    let mut segment_by_number = None;
    for row in rows {
        let expected = one_based_frame_number(prefix.len(), "可复用边界帧提示词")?;
        if row.shot_number == expected {
            let frame = match frame_prompts_from_rows(std::slice::from_ref(&row)) {
                Ok(mut frames) if frames.len() == 1 => frames.remove(0),
                Ok(frames) => {
                    tracing::info!(
                        target: "a4n::video_prompt",
                        prompt_id = row.id,
                        frame_count = frames.len(),
                        "已保存边界帧提示词不可复用，将从该帧开始重新生成"
                    );
                    break;
                }
                Err(err) => {
                    tracing::info!(
                        target: "a4n::video_prompt",
                        prompt_id = row.id,
                        error = %err,
                        "已保存边界帧提示词不可复用，将从该帧开始重新生成"
                    );
                    break;
                }
            };
            if units.is_none() {
                units = Some(units_for_frame_mapping(segment_plan)?);
                segment_by_number = Some(segment_index(segment_plan));
            }
            let expected_segment_numbers = boundary_frame_expected_segment_numbers_from_units(
                units.as_ref().expect("boundary frame units initialized"),
                frame.frame_number,
            )?;
            if let Err(err) = validate_single_boundary_frame_prompt_with_context(
                &frame,
                &expected_segment_numbers,
                segment_by_number
                    .as_ref()
                    .expect("boundary frame segment index initialized"),
                allowed_reference_ids,
                frame
                    .frame_number
                    .checked_sub(1)
                    .and_then(|number| image_id_by_frame_number.get(&number).copied()),
            ) {
                tracing::info!(
                    target: "a4n::video_prompt",
                    prompt_id = row.id,
                    frame_number = frame.frame_number,
                    error = %err,
                    "已保存边界帧提示词校验失败，将从该帧开始重新生成"
                );
                break;
            }
            prefix.push(row);
        } else if row.shot_number > expected {
            break;
        }
    }
    Ok(prefix)
}

fn segment_index(segment_plan: &[VideoSegmentPlan]) -> HashMap<i32, &VideoSegmentPlan> {
    segment_plan.iter().fold(
        HashMap::with_capacity(segment_plan.len()),
        |mut index, segment| {
            index.entry(segment.segment_number).or_insert(segment);
            index
        },
    )
}

fn boundary_frame_expected_duration_from_index(
    segment_numbers: &[i32],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
) -> Result<i32, AppError> {
    let mut duration = None;
    for segment_number in segment_numbers {
        let segment = segment_by_number
            .get(segment_number)
            .copied()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("边界帧引用了不存在的片段 {}", segment_number))
            })?;
        if duration.is_some_and(|expected| expected != segment.duration_secs) {
            return Err(AppError::InvalidArgument(
                "边界帧关联片段时长不一致".to_string(),
            ));
        }
        duration = Some(segment.duration_secs);
    }
    duration.ok_or_else(|| AppError::InvalidArgument("边界帧 segment_numbers 不能为空".to_string()))
}

fn one_based_frame_number(index: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(index)
        .map_err(|_| AppError::InvalidArgument(format!("{label}索引超过 i32 范围：{index}")))?
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArgument(format!("{label}编号超过 i32 范围")))
}
