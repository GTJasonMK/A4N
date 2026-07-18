mod entrypoints;
mod runner;

use crate::AppState;
use crate::actions::video_generation::VideoGenerationProgress;
use runner::wait_for_video_job_completion_inner;

pub(in crate::actions::video_generation) use entrypoints::{
    wait_for_canvas_graph_video_job_completion, wait_for_video_job_completion,
};
