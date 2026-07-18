use std::collections::HashSet;

use crate::video::prompt_records::VideoExecutionIdentityMatch;
use a4n_core::error::AppError;

use super::{
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, OrchestratorContext, PreparedVideoPromptPreviewInputs,
    PromptWorkflowMode, PromptWorkflowRowsContext, VideoFramePrompt, VideoSegmentPlan,
    first_frame_prompts_from_rows, initial_first_frame_rows, reusable_rows_are_complete,
    validate_first_frame_prompts,
};

impl OrchestratorContext {
    pub(super) async fn reusable_first_frame_prompts_for_preview(
        &self,
        project_id: i64,
        chapter_id: i64,
        input: &PreparedVideoPromptPreviewInputs,
        segment_plan: &[VideoSegmentPlan],
        allowed_reference_ids: &HashSet<i64>,
        execution_identity: &serde_json::Value,
        block_when_regenerating: bool,
    ) -> Result<Option<Vec<VideoFramePrompt>>, AppError> {
        let frame_rows = self
            .reusable_prompt_rows(
                PromptWorkflowRowsContext {
                    project_id,
                    chapter_id,
                    workflow_mode: PromptWorkflowMode::FirstFrameChain.as_str(),
                    prompt_type: "first_frame_prompt",
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
            .await?;
        let frame_rows = initial_first_frame_rows(frame_rows);
        if (block_when_regenerating && input.regenerate_existing)
            || !reusable_rows_are_complete(&frame_rows, FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT)
        {
            return Ok(None);
        }
        let first_frame_prompts = first_frame_prompts_from_rows(&frame_rows)?;
        validate_first_frame_prompts(&first_frame_prompts, segment_plan, allowed_reference_ids)?;
        Ok(Some(first_frame_prompts))
    }
}
