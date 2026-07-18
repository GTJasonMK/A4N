use super::final_video_prompt_public_context;

mod boundary_video;
mod chat_video;
mod contracts;
mod first_frame_video;
mod frame_context;
mod frame_prompts;
mod planning;
mod references;
mod rendering;
mod rules;
mod storyboard_video;

pub(in crate::video) use self::boundary_video::build_boundary_video_prompt_user;
pub(in crate::video) use self::chat_video::build_chat_video_prompt_user;
pub(in crate::video) use self::contracts::{
    FinalVideoPromptContext, FramePromptContext, VideoPlanningPromptContext,
};
pub(in crate::video) use self::first_frame_video::build_first_frame_video_prompt_user;
pub(in crate::video) use self::frame_prompts::{
    build_first_frame_prompt_user, build_single_frame_prompt_user, build_storyboard_prompt_user,
};
pub(in crate::video) use self::planning::build_segment_plan_for_shot_user;
pub(in crate::video) use self::storyboard_video::build_storyboard_video_prompt_user;

use self::contracts::{VideoPromptCommonConstraintInput, VideoPromptContractMode};
use self::frame_context::{
    add_input_frame_usage_constraint, focused_segment_plan, frame_prompt_context,
    lightweight_chapter_context, required_current_frame_field, storyboard_frame_for_segment,
    video_prompt_input_frames,
};
use self::references::{
    frame_reference_slot_order_rule, frame_reference_slots, reference_asset_summaries,
};
use self::rendering::{chapter_analysis_context, render_strict_json_user_prompt};
use self::rules::{
    final_video_prompt_output_contract, insert_rule_value, rule_text, rule_value,
    video_builder_rules, video_prompt_common_constraints, video_prompt_task,
};

#[cfg(test)]
mod tests;
