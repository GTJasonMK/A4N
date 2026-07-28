mod manual;
mod provider;

pub(in crate::actions::video_generation) use manual::poll_video_job;
pub(in crate::actions) use provider::poll_provider_job;
