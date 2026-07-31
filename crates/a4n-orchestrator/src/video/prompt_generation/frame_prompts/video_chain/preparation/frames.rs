use std::collections::HashMap;

use a4n_core::error::AppError;

use super::super::super::{
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, PromptWorkflowMode, available_video_image_ids,
    boundary_frame_image_ids_by_number, complete_reusable_rows, expected_boundary_frame_count,
    expected_video_unit_count, first_frame_prompts_from_rows, frame_prompts_from_rows,
    initial_first_frame_rows, reusable_frame_rows_for_video_prompt_inputs,
    storyboard_frame_image_ids_by_number, storyboard_prompts_from_rows,
    validate_first_frame_prompts, validate_frame_prompts,
    validate_frame_prompts_for_video_prompt_inputs, validate_storyboard_prompt_for_unit,
    validate_storyboard_prompts, video_unit_for_target_segment, video_units_for_segment_plan,
};
use super::super::inputs::select_boundary_video_prompt_inputs;
use super::contracts::{SelectedVideoChainFrames, VideoChainFrameSelectionInput};

pub(super) async fn select_video_chain_frames(
    input: VideoChainFrameSelectionInput<'_>,
) -> Result<SelectedVideoChainFrames, AppError> {
    let VideoChainFrameSelectionInput {
        service,
        spec,
        project_id,
        chapter_id,
        segment_plan,
        target_shot_number,
        duration_secs,
        execution_identity,
        prompt_output_language,
        available_assets,
        reference_image_ids,
        include_system_frame_images,
    } = input;
    let mut allowed_reference_ids =
        available_video_image_ids(available_assets, reference_image_ids)?;
    let targeted_boundary_segment_plan = if spec.workflow_mode == PromptWorkflowMode::BoundaryFrames
    {
        target_shot_number
            .map(|target| {
                segment_plan
                    .iter()
                    .find(|segment| segment.segment_number == target)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::InvalidArgument(format!("片段规划中不存在目标片段 {target}"))
                    })
            })
            .transpose()?
            .map(|segment| vec![segment])
    } else {
        None
    };
    let frame_validation_segment_plan = targeted_boundary_segment_plan
        .as_deref()
        .unwrap_or(segment_plan);
    let all_video_units = video_units_for_segment_plan(segment_plan, duration_secs)?;
    let prompt_video_units = if let Some(target) = target_shot_number {
        vec![video_unit_for_target_segment(
            segment_plan,
            duration_secs,
            target,
        )?]
    } else {
        all_video_units.clone()
    };
    let operation_video_units = if target_shot_number.is_some() {
        prompt_video_units.clone()
    } else {
        all_video_units.clone()
    };
    if skips_system_frame_prompt_inputs(spec.workflow_mode, target_shot_number) {
        return Ok(SelectedVideoChainFrames {
            frame_prompts: Vec::new(),
            prompt_video_units,
            all_video_units: operation_video_units,
            frame_image_ids_by_number: HashMap::new(),
        });
    }
    let frame_rows = reusable_frame_rows_for_video_prompt_inputs(
        service,
        project_id,
        chapter_id,
        spec.workflow_mode,
        spec.frame_prompt_type,
        spec.frame_source_prompt_type(),
        execution_identity,
        prompt_output_language,
    )
    .await?;
    let frame_rows = if spec.is_first_frame_chain() {
        initial_first_frame_rows(frame_rows)
    } else if let Some(target) = target_shot_number {
        frame_rows
            .into_iter()
            .filter(|row| {
                if spec.is_storyboard() {
                    row.shot_number == target
                } else {
                    row.shot_number == 1 || row.shot_number == 2
                }
            })
            .collect()
    } else {
        frame_rows
    };
    let mut frame_image_ids_by_number = if !include_system_frame_images {
        HashMap::new()
    } else if spec.workflow_mode == PromptWorkflowMode::BoundaryFrames {
        boundary_frame_image_ids_by_number(service, project_id, chapter_id, &frame_rows).await?
    } else if spec.workflow_mode == PromptWorkflowMode::Storyboard {
        storyboard_frame_image_ids_by_number(service, project_id, chapter_id, &frame_rows).await?
    } else {
        HashMap::new()
    };
    if spec.workflow_mode == PromptWorkflowMode::BoundaryFrames && target_shot_number.is_some() {
        let unit = &prompt_video_units[0];
        let start = unit
            .start_frame_number
            .ok_or_else(|| AppError::InvalidArgument("目标片段缺少起始边界帧编号".to_string()))?;
        let end = unit
            .end_frame_number
            .ok_or_else(|| AppError::InvalidArgument("目标片段缺少结束边界帧编号".to_string()))?;
        frame_image_ids_by_number = [
            (start, frame_image_ids_by_number.get(&1).copied()),
            (end, frame_image_ids_by_number.get(&2).copied()),
        ]
        .into_iter()
        .filter_map(|(number, image_id)| image_id.map(|image_id| (number, image_id)))
        .collect();
    }
    allowed_reference_ids.extend(frame_image_ids_by_number.values().copied());
    if include_system_frame_images && spec.is_storyboard() {
        let missing = prompt_video_units
            .iter()
            .filter(|unit| {
                unit.storyboard_number
                    .is_none_or(|number| !frame_image_ids_by_number.contains_key(&number))
            })
            .map(|unit| unit.unit_number.to_string())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "当前章节分镜板不完整：缺少第 {} 个视频单元分镜板，请先在分镜生成里生成对应图片",
                missing.join("、")
            )));
        }
    }
    let (frame_prompts, prompt_video_units) = if spec.is_first_frame_chain() {
        let expected_frame_count = FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT;
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(incomplete_frame_error(
                spec,
                expected_frame_count,
                actual_frame_count,
            ));
        };
        let prompts = first_frame_prompts_from_rows(&frame_rows)?;
        let prompts = validate_frame_prompts_for_video_prompt_inputs(
            prompts,
            segment_plan,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
        )?;
        validate_first_frame_prompts(&prompts, segment_plan, &allowed_reference_ids)?;
        (prompts, prompt_video_units.clone())
    } else if spec.workflow_mode == PromptWorkflowMode::BoundaryFrames {
        let (prompts, units) = select_boundary_video_prompt_inputs(
            frame_rows,
            frame_validation_segment_plan,
            duration_secs,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
            target_shot_number,
        )?;
        if units.len() != prompt_video_units.len() {
            return Err(AppError::InvalidArgument(format!(
                "当前片段{}不完整：请先生成目标片段所需的首尾帧提示词",
                spec.incomplete_frame_label
            )));
        }
        (prompts, units)
    } else if spec.is_storyboard() {
        let expected_frame_count = if target_shot_number.is_some() {
            1
        } else {
            expected_video_unit_count(segment_plan)
        };
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(incomplete_frame_error(
                spec,
                expected_frame_count,
                actual_frame_count,
            ));
        };
        let prompts = storyboard_prompts_from_rows(&frame_rows)?;
        if target_shot_number.is_some() {
            validate_storyboard_prompt_for_unit(
                &prompts[0],
                segment_plan,
                &prompt_video_units[0],
                &allowed_reference_ids,
            )?;
        } else {
            validate_storyboard_prompts(&prompts, segment_plan, &allowed_reference_ids)?;
        }
        (prompts, prompt_video_units.clone())
    } else {
        let expected_frame_count = expected_boundary_frame_count(segment_plan);
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(incomplete_frame_error(
                spec,
                expected_frame_count,
                actual_frame_count,
            ));
        };
        let prompts = frame_prompts_from_rows(&frame_rows)?;
        let prompts = validate_frame_prompts_for_video_prompt_inputs(
            prompts,
            segment_plan,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
        )?;
        validate_frame_prompts(&prompts, segment_plan, &allowed_reference_ids)?;
        (prompts, prompt_video_units.clone())
    };
    Ok(SelectedVideoChainFrames {
        frame_prompts,
        prompt_video_units,
        all_video_units: operation_video_units,
        frame_image_ids_by_number,
    })
}

fn skips_system_frame_prompt_inputs(
    workflow_mode: PromptWorkflowMode,
    target_shot_number: Option<i32>,
) -> bool {
    workflow_mode == PromptWorkflowMode::ChatVideo
        || (workflow_mode == PromptWorkflowMode::FirstFrameChain
            && target_shot_number.is_some_and(|target| target > 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_video_never_requires_system_frame_prompt_inputs() {
        assert!(skips_system_frame_prompt_inputs(
            PromptWorkflowMode::ChatVideo,
            None
        ));
        assert!(skips_system_frame_prompt_inputs(
            PromptWorkflowMode::ChatVideo,
            Some(3)
        ));
        assert!(!skips_system_frame_prompt_inputs(
            PromptWorkflowMode::BoundaryFrames,
            Some(1)
        ));
    }
}

fn incomplete_frame_error(
    spec: &super::super::super::FramePromptGenerationSpec,
    expected_frame_count: usize,
    actual_frame_count: usize,
) -> AppError {
    AppError::InvalidArgument(format!(
        "当前章节{}不完整：需要 {} 条，当前 {} 条，请先生成完整{}",
        spec.incomplete_frame_label,
        expected_frame_count,
        actual_frame_count,
        spec.incomplete_frame_label
    ))
}
