mod capacity;
mod config;

pub(super) use capacity::{ensure_created_video_capacity, ensure_video_capacity};
pub(super) use config::resolve_video_config;
