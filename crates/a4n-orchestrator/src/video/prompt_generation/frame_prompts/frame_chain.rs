mod boundary_frames;
mod boundary_workflow;
mod contracts;
mod finalization;
mod first_frame_workflow;
mod frame_preparation;
mod layout_dispatch;
mod orchestration;
mod planning_workflow;
mod segment_plan;
mod shooting_script_projection;
mod storyboard_workflow;

pub(super) use self::contracts::{FramePromptChainInput, SegmentPlanForSpecInput};
pub(super) use self::orchestration::generate_chapter_frame_prompt_chain;
pub(super) use self::segment_plan::generate_chapter_segment_plan_for_spec;
