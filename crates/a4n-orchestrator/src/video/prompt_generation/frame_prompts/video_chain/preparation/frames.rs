use std::collections::HashMap;

use a4n_core::error::AppError;

use super::super::super::{
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, PromptWorkflowMode, available_video_image_ids,
    boundary_frame_image_ids_by_number, complete_reusable_rows, expected_boundary_frame_count,
    expected_video_unit_count, first_frame_prompts_from_rows, frame_prompts_from_rows,
    initial_first_frame_rows, reusable_frame_rows_for_video_prompt_inputs,
    storyboard_frame_image_ids_by_number, storyboard_prompts_from_rows,
    validate_first_frame_prompts, validate_frame_prompts,
    validate_frame_prompts_for_video_prompt_inputs, validate_storyboard_prompts,
    video_units_for_segment_plan,
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
        duration_secs,
        execution_identity,
        prompt_output_language,
        available_assets,
        reference_image_ids,
    } = input;
    let mut allowed_reference_ids =
        available_video_image_ids(available_assets, reference_image_ids)?;
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
    } else {
        frame_rows
    };
    let frame_image_ids_by_number = if matches!(
        spec.workflow_mode,
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo
    ) {
        boundary_frame_image_ids_by_number(service, project_id, chapter_id, &frame_rows).await?
    } else if spec.workflow_mode == PromptWorkflowMode::Storyboard {
        storyboard_frame_image_ids_by_number(service, project_id, chapter_id, &frame_rows).await?
    } else {
        HashMap::new()
    };
    allowed_reference_ids.extend(frame_image_ids_by_number.values().copied());
    let all_video_units = video_units_for_segment_plan(segment_plan, duration_secs)?;
    if spec.is_storyboard() {
        let missing = all_video_units
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
        (prompts, all_video_units.clone())
    } else if matches!(
        spec.workflow_mode,
        PromptWorkflowMode::BoundaryFrames | PromptWorkflowMode::ChatVideo
    ) {
        select_boundary_video_prompt_inputs(
            frame_rows,
            segment_plan,
            duration_secs,
            &allowed_reference_ids,
            spec.incomplete_frame_label,
        )?
    } else if spec.is_storyboard() {
        let expected_frame_count = expected_video_unit_count(segment_plan);
        let actual_frame_count = frame_rows.len();
        let Some(frame_rows) = complete_reusable_rows(frame_rows, expected_frame_count) else {
            return Err(incomplete_frame_error(
                spec,
                expected_frame_count,
                actual_frame_count,
            ));
        };
        let prompts = storyboard_prompts_from_rows(&frame_rows)?;
        validate_storyboard_prompts(&prompts, segment_plan, &allowed_reference_ids)?;
        (prompts, all_video_units.clone())
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
        (prompts, all_video_units.clone())
    };
    Ok(SelectedVideoChainFrames {
        frame_prompts,
        prompt_video_units,
        all_video_units,
        frame_image_ids_by_number,
    })
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
