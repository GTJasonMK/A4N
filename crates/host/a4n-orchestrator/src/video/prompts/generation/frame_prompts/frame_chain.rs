mod boundary_frames;
mod boundary_workflow;
mod contracts;
mod finalization;
mod first_frame_workflow;
mod frame_preparation;
mod layout_dispatch;
mod orchestration;
mod segment_plan;
mod shooting_script_projection;
mod storyboard_workflow;

pub(super) use self::contracts::{FramePromptChainInput, SharedSegmentPlanInput};
pub(super) use self::orchestration::generate_chapter_frame_prompt_chain;
pub(super) use self::segment_plan::generate_shared_segment_plan;
pub(super) use self::segment_plan::{
    SegmentPlanDraftInput, merge_incremental_segment_plan, segment_plan_covers_required_shots,
    segment_plan_draft,
};
