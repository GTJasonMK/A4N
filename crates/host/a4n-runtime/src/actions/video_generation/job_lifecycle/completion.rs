mod failure;
mod video;

pub(in crate::actions) use failure::cleanup_saved_video_outputs;
pub(in crate::actions::video_generation) use video::persist_video_result;
