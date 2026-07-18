use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::{
    super::{
        OrchestratorContext, PreparedVideoPromptPreviewInputs, PromptWorkflowMode,
        PromptWorkflowRowsContext, VideoFramePrompt, VideoSegmentPlan, expected_video_unit_count,
        frame_prompt_available_assets_context, storyboard_prompts_from_rows,
        validate_storyboard_prompts,
    },
    one_based_preview_number,
};
use crate::video::prompt_generation::video_units_for_segment_plan;
use crate::video::prompt_records::VideoExecutionIdentityMatch;

pub(in crate::video::prompt_preview::local) enum StoryboardPromptPreviewState {
    Complete(Vec<VideoFramePrompt>),
    Next(StoryboardPromptPreviewNext),
}

pub(in crate::video::prompt_preview::local) struct StoryboardPromptPreviewNext {
    pub(in crate::video::prompt_preview::local) current_frame: Value,
    pub(in crate::video::prompt_preview::local) available_assets: Value,
}

impl OrchestratorContext {
    pub(in crate::video::prompt_preview::local) async fn storyboard_prompt_preview_state(
        &self,
        project_id: i64,
        chapter_id: i64,
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        allowed_reference_ids: &HashSet<i64>,
        execution_identity: &Value,
    ) -> Result<StoryboardPromptPreviewState, AppError> {
        if segment_plan.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频片段规划不能为空".to_string(),
            ));
        }

        let reusable_rows = if input.regenerate_existing {
            Vec::new()
        } else {
            self.reusable_prompt_rows(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::Storyboard.as_str(),
                    prompt_type: "storyboard_prompt",
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    duration_secs: input.duration_secs,
                    aspect_ratio: &input.aspect_ratio,
                    reference_ids: &input.reference_ids,
                    generation_asset_fingerprint: Some(&input.generation_asset_fingerprint),
                    source_prompt_type: Some("frame_segment_plan"),
                    video_execution_identity: VideoExecutionIdentityMatch::Exact(
                        execution_identity,
                    ),
                },
                input.prompt_output_language,
            )
            .await?
        };

        let expected_count = expected_video_unit_count(segment_plan);
        let mut prefix_rows = Vec::new();
        for row in reusable_rows {
            if prefix_rows.len() >= expected_count {
                break;
            }
            let expected_number = one_based_preview_number(prefix_rows.len(), "可复用分镜预览")?;
            if row.shot_number == expected_number {
                if storyboard_prompts_from_rows(std::slice::from_ref(&row)).is_err() {
                    break;
                }
                prefix_rows.push(row);
            } else if row.shot_number > expected_number {
                break;
            }
        }

        let storyboard_prompts = storyboard_prompts_from_rows(&prefix_rows)?;
        if storyboard_prompts.len() == expected_count {
            validate_storyboard_prompts(&storyboard_prompts, segment_plan, allowed_reference_ids)?;
            return Ok(StoryboardPromptPreviewState::Complete(storyboard_prompts));
        }

        Ok(StoryboardPromptPreviewState::Next(
            Self::storyboard_prompt_preview_next(input, segment_plan, storyboard_prompts.len())?,
        ))
    }

    pub(in crate::video::prompt_preview::local) fn placeholder_storyboard_prompt_preview_state(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
    ) -> Result<StoryboardPromptPreviewNext, AppError> {
        Self::storyboard_prompt_preview_next(input, segment_plan, 0)
    }

    fn storyboard_prompt_preview_next(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        completed_count: usize,
    ) -> Result<StoryboardPromptPreviewNext, AppError> {
        let expected_frame_count = expected_video_unit_count(segment_plan);
        if completed_count >= expected_frame_count {
            return Err(AppError::InvalidArgument(format!(
                "分镜提示词预览已完成 {completed_count}/{expected_frame_count} 个视频单元，没有待生成分镜"
            )));
        }
        let unit_number = one_based_preview_number(completed_count, "分镜提示词预览")?;
        let duration_secs = segment_plan
            .first()
            .map(|segment| segment.duration_secs)
            .ok_or_else(|| AppError::InvalidArgument("分镜提示词预览缺少片段规划".to_string()))?;
        let units = video_units_for_segment_plan(segment_plan, duration_secs)?;
        let unit = units.get(completed_count).ok_or_else(|| {
            AppError::InvalidArgument("分镜提示词预览缺少待生成视频单元".to_string())
        })?;
        let expected_segment_numbers = unit.source_segment_numbers();
        let segment_number = expected_segment_numbers.first().copied().ok_or_else(|| {
            AppError::InvalidArgument("分镜提示词预览缺少待生成视频单元源片段".to_string())
        })?;
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument("分镜提示词预览缺少待生成视频单元".to_string())
            })?;
        let next_segment = expected_segment_numbers.get(1).and_then(|segment_number| {
            segment_plan
                .iter()
                .find(|segment| segment.segment_number == *segment_number)
        });
        let current_frame = json!({
            "frame_number": unit_number,
            "total_frame_count": expected_frame_count,
            "role": if unit.is_action_bridge() { "action_bridge_storyboard" } else { "content_storyboard" },
            "video_unit": unit,
            "expected_segment_numbers": expected_segment_numbers,
            "required_duration_secs": segment.duration_secs,
            "required_aspect_ratio": &segment.aspect_ratio,
            "previous_segment": if unit.is_action_bridge() { Some(segment) } else { None },
            "current_segment": if unit.is_action_bridge() { None } else { Some(segment) },
            "next_segment": next_segment,
            "reference_slot_instruction": "后端提供的 constraints.reference_slots 顺序就是本帧成图输入顺序；frame_prompt 只能按槽位顺序写参考图 1、参考图 2，不要输出 reference_image_ids，不得写 image_id、图片 ID、available_assets、候选素材标签或原始图片数字 ID。",
            "continuity_instruction": "这是当前视频片段的多格分镜板，用于表达本段核心构图、动作姿态、空间关系和光影，不是首帧或尾帧。"
        });
        Ok(StoryboardPromptPreviewNext {
            current_frame,
            available_assets: frame_prompt_available_assets_context(&input.available_assets)?,
        })
    }
}
