mod boundary_context;
mod boundary_sync;
mod checkpoint;
mod frame_chain;
mod generation_spec;
mod inserted_segment;
mod parsing;
mod prepared;
mod progress;
mod records;
mod render_restore;
mod reuse;
mod shooting_script_projection;
mod staged_segment_plan;
#[cfg(test)]
mod tests;
mod validation;
mod video_chain;
mod video_units;

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_error::AppError;
use serde_json::{Value, json};

#[cfg(test)]
use super::super::super::BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION;
use super::super::super::library::{available_video_image_ids, ensure_video_reference_ids_known};
use super::super::super::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSourceContext,
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};
use super::super::context::{
    FinalVideoPromptContext, FramePromptContext, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
    VideoContextManifestInput, VideoContextPreviousOutput, VideoPlanningPromptContext,
    build_boundary_video_prompt_user, build_chat_video_prompt_user,
    build_final_video_context_manifest, build_first_frame_prompt_user,
    build_first_frame_video_prompt_user, build_segment_plan_for_shot_user,
    build_storyboard_prompt_user, build_storyboard_video_prompt_user, build_video_context_manifest,
    validate_prompt_options, video_source_metadata,
};
use super::super::json::{
    ChatVideoPromptShot, VideoCinematicShot, VideoFramePrompt, VideoFramePromptDefaults,
    VideoPromptShot, VideoSegmentPlan, VideoShootingScriptProjection,
    parse_chat_video_prompt_shots, parse_storyboard_frame_prompts_with_defaults,
    parse_video_frame_prompts_with_defaults, parse_video_prompt_shots,
    parse_video_segment_plan_response, validate_direct_prompt_text, validate_reference_label_text,
    validate_segment_plan_against_shooting_script_projection, video_segment_plan_input_values,
};
use super::super::records::{PromptWorkflowRowsContext, video_generation_asset_fingerprint};
use super::super::store::{NewChapterVideoPrompt, PromptWorkflowMode, VideoPromptGenerationTrace};
use super::{
    FieldMilestone, ProgressSpan, RenderedVideoPromptTrace, StructuredJsonProgressSpec,
    VideoPromptLlmCallTraceContext, VideoPromptRenderInput, VideoPromptStepProgress,
    build_video_prompt_request_with_call_trace, render_and_chat_trace_with_context,
    report_video_prompt_checkpoint, report_video_prompt_item_committed,
    report_video_prompt_progress,
};
use a4n_domain::source_spans::SourceSpan;

use self::boundary_context::apply_boundary_frame_style_template;
#[cfg(test)]
use self::boundary_context::frame_prompt_with_style_template;
pub(in crate::video) use self::boundary_context::{
    available_assets_with_previous_boundary_frame, backend_frame_reference_ids,
    boundary_frame_generation_context, frame_prompt_available_assets_context,
    previous_boundary_frame_asset_context, reference_slots_for_frame,
};
pub(in crate::video) use self::boundary_sync::reusable_frame_prefix_len_with_previous_images;
use self::boundary_sync::*;
use self::checkpoint::*;
pub(in crate::video) use self::checkpoint::{
    video_execution_chain_identity, video_segment_execution_identity,
    video_segment_frame_execution_identity,
};
use self::frame_chain::{
    FramePromptChainInput, SharedSegmentPlanInput, generate_chapter_frame_prompt_chain,
    generate_shared_segment_plan,
};
use self::generation_spec::*;
pub(in crate::video) use self::inserted_segment::generate_inserted_segment_draft;
pub(crate) use self::inserted_segment::validate_video_inserted_segment_generation_options;
use self::parsing::*;
use self::prepared::{
    LoadedSharedSegmentPlan, PreparedSegmentPlan, SHARED_SEGMENT_PLAN_CONTEXT,
    SHARED_SEGMENT_PLAN_CONTRACT_VERSION, SHARED_SEGMENT_PLAN_PROGRESS, SHARED_SEGMENT_PLAN_TITLE,
};
pub(in crate::video) use self::prepared::{PreparedVideoPromptInputs, ReusableSegmentPlan};
use self::progress::*;
use self::records::{
    ChatVideoPromptDraftsInput, FirstFramePromptDraftsInput, FirstFrameVideoPromptDraftsInput,
    FramePromptDraftsInput, StoryboardPromptDraftsInput, StoryboardVideoPromptDraftsInput,
    VideoPromptDraftCommonInput, VideoPromptDraftsInput, chat_video_prompt_drafts,
    complete_reusable_rows, first_frame_prompt_drafts, first_frame_video_prompt_drafts,
    frame_prompt_drafts, prompt_row_draft, storyboard_prompt_drafts,
    storyboard_video_prompt_drafts, validate_chat_video_prompts,
    validate_partial_chat_video_prompts, validate_partial_video_prompts, validate_video_prompts,
    video_prompt_drafts,
};
pub(in crate::video) use self::records::{
    first_frame_prompts_from_rows, frame_prompts_from_rows, initial_first_frame_rows,
    reusable_boundary_video_prompts_from_rows, reusable_chat_video_prompts_from_rows,
    reusable_first_frame_video_prompts_from_rows, storyboard_prompts_from_rows,
    validate_first_frame_prompts, validate_frame_prompts, validate_storyboard_prompt,
    validate_storyboard_prompt_for_unit, validate_storyboard_prompts,
};
use self::render_restore::*;
use self::reuse::*;
pub(in crate::video) use self::reuse::{
    FrameImageLookupContext, FrameImageLookupRowsQuery, boundary_frame_image_ids_by_number,
    boundary_frame_rows_for_image_lookup, storyboard_frame_image_ids_by_number,
};
pub(in crate::video) use self::shooting_script_projection::video_projection_from_shooting_script;
use self::staged_segment_plan::{
    StagedSegmentPlanInput, load_previous_shared_segment_recipe_state,
    render_staged_segment_plan_with_duration, select_video_segment_recipe_state_by_shots,
};
pub(in crate::video) use self::staged_segment_plan::{
    load_video_segment_plan_draft_checkpoints, load_video_segment_plan_draft_recipe_state,
    update_video_segment_plan_draft_recipe_checkpoint,
    video_segment_recipes_from_draft_for_final_plan,
};
pub(in crate::video) use self::validation::boundary_frame_prompt_rows_prefix;
use self::validation::*;
use self::video_units::*;
pub(in crate::video) use self::video_units::{
    VideoUnitPlan, boundary_frame_expected_segment_numbers_from_units,
    expected_boundary_frame_count, expected_video_unit_count, storyboard_expected_segment_numbers,
    units_for_frame_mapping, video_unit_for_target_segment, video_units_for_segment_plan,
};

const FINAL_VIDEO_PROMPT_OUTPUT_CONTRACT_VERSION: &str =
    "final_video_prompt_strict_canonical_checkpoint_v2";
