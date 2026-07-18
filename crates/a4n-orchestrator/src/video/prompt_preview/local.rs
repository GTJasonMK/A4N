use a4n_prompt::rendering::PromptRenderer;

use super::preview_steps::{
    first_cinematic_shot_for_preview, placeholder_boundary_frame_context_for_video_prompt,
    placeholder_first_frame_prompts, placeholder_segment_plan, placeholder_segments,
    placeholder_step_from_request, placeholder_storyboard_prompts,
    preview_chat_video_prompt_from_steps, preview_storyboard_video_prompt_from_steps,
    preview_video_prompt_from_steps, reused_step, step_from_request,
};
use super::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};
use crate::video::prompt_context::{
    FramePromptContext, VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
    VideoContextManifestInput, VideoContextPreviousOutput, VideoPlanningPromptContext,
    build_final_video_context_manifest, build_first_frame_prompt_user,
    build_segment_plan_for_shot_user, build_single_frame_prompt_user, build_storyboard_prompt_user,
    build_video_context_manifest,
};
use crate::video::prompt_generation::{
    FrameImageLookupContext, FrameImageLookupRowsQuery,
    available_assets_with_previous_boundary_frame, backend_frame_reference_ids,
    boundary_frame_generation_context, boundary_frame_image_ids_by_number,
    boundary_frame_prompt_rows_prefix, boundary_frame_rows_for_image_lookup,
    build_video_prompt_request, expected_boundary_frame_count, expected_video_unit_count,
    first_frame_prompts_from_rows, frame_prompt_available_assets_context, frame_prompts_from_rows,
    initial_first_frame_rows, previous_boundary_frame_asset_context,
    reusable_frame_prefix_len_with_previous_images, storyboard_prompts_from_rows,
    validate_first_frame_prompts, validate_frame_prompts, validate_storyboard_prompts,
};
use crate::video::prompt_json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::prompt_records::{
    PromptWorkflowRowsContext, VideoExecutionIdentityMatch, reusable_rows_are_complete,
};
use crate::video::prompt_store::PromptWorkflowMode;
use crate::video::{
    ChapterVideoPromptOptions, ChapterVideoSourceContext, FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT,
    OrchestratorContext,
};

mod frame_workflow;
mod inputs;
mod planning;
mod requests;
mod reusable;
mod spec;
mod states;
mod video;
mod video_completion;
mod video_facade;
mod video_predecessors;

use self::inputs::{LocalPromptPreviewInput, PreparedVideoPromptPreviewInputs};
use self::spec::LocalVideoPromptPreviewSpec;
