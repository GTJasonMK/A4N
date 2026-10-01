use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use a4n_error::AppError;
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
    frame_rows: Vec<ChapterVideoPromptRecord>,
    segment_plan: &[VideoSegmentPlan],
    duration_secs: i32,
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
    target_segment_number: Option<i32>,
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

    let mut units = video_units_for_segment_plan(segment_plan, duration_secs)?;
    let target_end_frame_number = target_segment_number
        .map(|target| {
            target.checked_add(1).ok_or_else(|| {
                AppError::InvalidArgument("目标片段编号超过边界帧可表示范围".to_string())
            })
        })
        .transpose()?;
    if let Some(target_segment_number) = target_segment_number {
        if units.len() != 1 {
            return Err(AppError::InvalidArgument(
                "片段卡片首尾帧只能绑定一个目标视频单元".to_string(),
            ));
        }
        units[0].unit_number = target_segment_number;
        units[0].prompt_segment.segment_number = target_segment_number;
        units[0].start_frame_number = Some(1);
        units[0].end_frame_number = Some(2);
    }
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

    let mut selected_frames = prompts
        .into_iter()
        .filter(|frame| selected_frame_numbers.contains(&frame.frame_number))
        .collect::<Vec<_>>();
    if let (Some(target_segment_number), Some(target_end_frame_number)) =
        (target_segment_number, target_end_frame_number)
    {
        selected_units[0].start_frame_number = Some(target_segment_number);
        selected_units[0].end_frame_number = Some(target_end_frame_number);
        for frame in &mut selected_frames {
            frame.frame_number = match frame.frame_number {
                1 => target_segment_number,
                2 => target_end_frame_number,
                actual => {
                    return Err(AppError::InvalidArgument(format!(
                        "目标片段首尾帧只允许本地编号 1/2，实际为 {actual}"
                    )));
                }
            };
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::prompts::generation::frame_prompts::tests::{
        boundary_frame, row_for_boundary_frame, segment,
    };

    #[test]
    fn targeted_boundary_inputs_keep_target_segment_identity() {
        let target_segment = segment(2);
        let rows = vec![
            row_for_boundary_frame(1, boundary_frame(1, Vec::new(), vec![2])),
            row_for_boundary_frame(2, boundary_frame(2, Vec::new(), vec![2])),
        ];

        let (frames, units) = select_boundary_video_prompt_inputs(
            rows,
            std::slice::from_ref(&target_segment),
            target_segment.duration_secs,
            &HashSet::new(),
            "边界帧提示词",
            Some(2),
        )
        .expect("target segment 2 should use its two local boundary frames");

        assert_eq!(
            frames
                .iter()
                .map(|frame| frame.frame_number)
                .collect::<Vec<_>>(),
            [2, 3]
        );
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].unit_number, 2);
        assert_eq!(units[0].prompt_segment.segment_number, 2);
        assert_eq!(units[0].source_segment_numbers(), [2]);
        assert_eq!(units[0].start_frame_number, Some(2));
        assert_eq!(units[0].end_frame_number, Some(3));
    }
}
