mod frame_prompts;
mod llm_chat;

#[cfg(test)]
pub(in crate::video) use self::frame_prompts::video_execution_chain_identity;
pub(in crate::video) use self::frame_prompts::{
    FrameImageLookupContext, FrameImageLookupRowsQuery, PreparedVideoPromptInputs,
    ReusableSegmentPlan, ReusableSegmentPlanForPreviewQuery, VideoUnitPlan,
    available_assets_with_previous_boundary_frame, backend_frame_reference_ids,
    boundary_frame_generation_context, boundary_frame_image_ids_by_number,
    boundary_frame_prompt_rows_prefix, boundary_frame_rows_for_image_lookup,
    expected_boundary_frame_count, expected_video_unit_count, first_frame_prompts_from_rows,
    frame_prompt_available_assets_context, frame_prompts_from_rows, initial_first_frame_rows,
    load_previous_shared_segment_recipe_state, materialize_video_segment_plan_draft_recipes,
    previous_boundary_frame_asset_context, reusable_boundary_video_prompts_from_rows,
    reusable_chat_video_prompts_from_rows, reusable_first_frame_video_prompts_from_rows,
    reusable_frame_prefix_len_with_previous_images, reusable_segment_plan_for_preview,
    storyboard_expected_segment_numbers, storyboard_prompts_from_rows,
    update_video_segment_plan_draft_recipe_checkpoint, validate_first_frame_prompts,
    validate_frame_prompts, validate_storyboard_prompts, video_projection_from_shooting_script,
    video_prompt_segments_with_reference_ids, video_segment_execution_identity,
    video_units_for_segment_plan,
};
pub(in crate::video) use self::llm_chat::build_video_prompt_request;
use self::llm_chat::{
    RenderedVideoPromptTrace, VideoPromptLlmCallTraceContext, VideoPromptRenderInput,
    VideoPromptStepProgress, build_video_prompt_request_with_call_trace,
    render_and_chat_trace_with_context, report_video_prompt_checkpoint,
    report_video_prompt_item_committed, report_video_prompt_progress,
};
use a4n_core::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
