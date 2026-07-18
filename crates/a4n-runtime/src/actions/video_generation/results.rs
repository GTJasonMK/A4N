mod canvas;
mod chapter;
mod contracts;
mod failure;
mod generation;

pub(super) use self::canvas::{
    handle_canvas_video_provider_result, save_completed_canvas_video_result,
};
pub(super) use self::chapter::handle_video_provider_result;
pub(super) use self::contracts::VideoProviderResultHandling;
pub(super) use self::failure::{
    ensure_provider_generation_run_updated, fail_video_generation_runs,
    mark_video_generation_failed,
};
pub(super) use self::generation::started_video_generation_run_input;
