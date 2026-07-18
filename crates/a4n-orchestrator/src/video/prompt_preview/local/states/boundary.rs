use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::{
    super::{
        FrameImageLookupContext, FrameImageLookupRowsQuery, OrchestratorContext,
        PreparedVideoPromptPreviewInputs, PromptWorkflowMode, PromptWorkflowRowsContext,
        VideoExecutionIdentityMatch, VideoFramePrompt, VideoSegmentPlan,
        available_assets_with_previous_boundary_frame, backend_frame_reference_ids,
        boundary_frame_generation_context, boundary_frame_image_ids_by_number,
        boundary_frame_prompt_rows_prefix, boundary_frame_rows_for_image_lookup,
        expected_boundary_frame_count, frame_prompt_available_assets_context,
        frame_prompts_from_rows, previous_boundary_frame_asset_context,
        reusable_frame_prefix_len_with_previous_images, validate_frame_prompts,
    },
    one_based_preview_number,
};

pub(in crate::video::prompt_preview::local) enum BoundaryFramePromptPreviewState {
    Complete(Vec<VideoFramePrompt>),
    Next(BoundaryFramePromptPreviewNext),
}

pub(in crate::video::prompt_preview::local) struct BoundaryFramePromptPreviewNext {
    pub(in crate::video::prompt_preview::local) current_frame: Value,
    pub(in crate::video::prompt_preview::local) available_assets: Value,
    pub(in crate::video::prompt_preview::local) reference_ids: Value,
}

pub(in crate::video::prompt_preview::local) struct BoundaryFramePromptPreviewStateInput<'a> {
    pub(in crate::video::prompt_preview::local) project_id: i64,
    pub(in crate::video::prompt_preview::local) chapter_id: i64,
    pub(in crate::video::prompt_preview::local) input: &'a PreparedVideoPromptPreviewInputs,
    pub(in crate::video::prompt_preview::local) segment_plan: &'a [VideoSegmentPlan],
    pub(in crate::video::prompt_preview::local) allowed_reference_ids: &'a HashSet<i64>,
    pub(in crate::video::prompt_preview::local) workflow_mode: PromptWorkflowMode,
    pub(in crate::video::prompt_preview::local) frame_prompt_type: &'a str,
    pub(in crate::video::prompt_preview::local) frame_source_prompt_type: &'a str,
    pub(in crate::video::prompt_preview::local) execution_identity: &'a Value,
}

impl OrchestratorContext {
    pub(in crate::video::prompt_preview::local) async fn boundary_frame_prompt_preview_state(
        &self,
        input: BoundaryFramePromptPreviewStateInput<'_>,
    ) -> Result<BoundaryFramePromptPreviewState, AppError> {
        let BoundaryFramePromptPreviewStateInput {
            project_id,
            chapter_id,
            input,
            segment_plan,
            allowed_reference_ids,
            workflow_mode,
            frame_prompt_type,
            frame_source_prompt_type,
            execution_identity,
        } = input;
        if segment_plan.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频片段规划不能为空".to_string(),
            ));
        }

        let frame_rows = if input.regenerate_existing {
            Vec::new()
        } else {
            self.reusable_prompt_rows(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: workflow_mode.as_str(),
                    prompt_type: frame_prompt_type,
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    duration_secs: input.duration_secs,
                    aspect_ratio: &input.aspect_ratio,
                    reference_ids: &input.reference_ids,
                    generation_asset_fingerprint: Some(&input.generation_asset_fingerprint),
                    source_prompt_type: Some(frame_source_prompt_type),
                    video_execution_identity: VideoExecutionIdentityMatch::Exact(
                        execution_identity,
                    ),
                },
                input.prompt_output_language,
            )
            .await?
        };
        let frame_image_lookup_rows = if input.regenerate_existing {
            Vec::new()
        } else {
            boundary_frame_rows_for_image_lookup(FrameImageLookupRowsQuery {
                service: self,
                project_id,
                chapter_id,
                context: FrameImageLookupContext {
                    workflow_mode,
                    prompt_type: frame_prompt_type,
                    source_metadata: &input.source_metadata,
                    style: &input.style,
                    aspect_ratio: &input.aspect_ratio,
                    video_execution_identity: execution_identity,
                    source_prompt_type: frame_source_prompt_type,
                    prompt_output_language: input.prompt_output_language,
                },
            })
            .await?
        };
        let frame_image_id_by_number = boundary_frame_image_ids_by_number(
            self,
            project_id,
            chapter_id,
            &frame_image_lookup_rows,
        )
        .await?;
        let mut allowed_boundary_reference_ids = allowed_reference_ids.clone();
        allowed_boundary_reference_ids.extend(frame_image_id_by_number.values().copied());
        let prefix_rows = boundary_frame_prompt_rows_prefix(
            frame_rows,
            segment_plan,
            &allowed_boundary_reference_ids,
            &frame_image_id_by_number,
        )?;
        let mut completed_frames = frame_prompts_from_rows(&prefix_rows)?;
        let reusable_prefix_len = reusable_frame_prefix_len_with_previous_images(
            &completed_frames,
            &frame_image_id_by_number,
        );
        completed_frames.truncate(reusable_prefix_len);

        let expected_frame_count = expected_boundary_frame_count(segment_plan);
        if completed_frames.len() == expected_frame_count {
            validate_frame_prompts(
                &completed_frames,
                segment_plan,
                &allowed_boundary_reference_ids,
            )?;
            return Ok(BoundaryFramePromptPreviewState::Complete(completed_frames));
        }

        Ok(BoundaryFramePromptPreviewState::Next(
            Self::boundary_frame_prompt_preview_next(
                input,
                segment_plan,
                completed_frames,
                &frame_image_id_by_number,
            )?,
        ))
    }

    pub(in crate::video::prompt_preview::local) fn placeholder_boundary_frame_prompt_preview_state(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
    ) -> Result<BoundaryFramePromptPreviewNext, AppError> {
        Self::boundary_frame_prompt_preview_next(input, segment_plan, Vec::new(), &HashMap::new())
    }

    fn boundary_frame_prompt_preview_next(
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        completed_frames: Vec<VideoFramePrompt>,
        frame_image_id_by_number: &HashMap<i32, i64>,
    ) -> Result<BoundaryFramePromptPreviewNext, AppError> {
        if segment_plan.is_empty() {
            return Err(AppError::InvalidArgument(
                "视频片段规划不能为空".to_string(),
            ));
        }
        let frame_number = one_based_preview_number(completed_frames.len(), "边界帧预览")?;
        let previous_frame_image_id = if frame_number > 1 {
            frame_image_id_by_number.get(&(frame_number - 1)).copied()
        } else {
            None
        };
        let available_assets = available_assets_with_previous_boundary_frame(
            &input.available_assets,
            previous_frame_image_id,
            completed_frames.last(),
        );
        let prompt_available_assets = frame_prompt_available_assets_context(&available_assets)?;
        let current_frame = boundary_frame_generation_context(
            segment_plan,
            &completed_frames,
            frame_number,
            previous_boundary_frame_asset_context(&available_assets, previous_frame_image_id),
        )?;

        Ok(BoundaryFramePromptPreviewNext {
            current_frame,
            available_assets: prompt_available_assets,
            reference_ids: json!(backend_frame_reference_ids(
                &input.reference_image_ids,
                previous_frame_image_id,
            )),
        })
    }
}
