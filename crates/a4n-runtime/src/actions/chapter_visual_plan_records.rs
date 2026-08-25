mod contracts;
mod generation;
mod read;
mod write;

pub use contracts::*;
pub use generation::{
    generate_chapter_visual_plan_draft, preview_chapter_visual_plan_request,
    propose_chapter_visual_plan_revision,
};
pub use read::get_chapter_visual_plan;
pub use write::{
    apply_chapter_visual_plan_proposal, confirm_chapter_visual_plan,
    discard_chapter_visual_plan_draft, save_chapter_visual_plan_draft,
};

pub const CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL: u32 =
    a4n_orchestrator::chapter_visual_plan::CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL;
