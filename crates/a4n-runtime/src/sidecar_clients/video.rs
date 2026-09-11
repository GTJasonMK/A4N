mod contracts;
mod events;
mod operations;

pub(crate) use operations::{
    delete_video_job, generate_video, poll_video, preview_video_provider_request, video_job_status,
};

#[cfg(test)]
use crate::state::RuntimeProfile;
#[cfg(test)]
use a4n_capabilities::A4nCapabilities;
#[cfg(test)]
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};
#[cfg(test)]
use a4n_video::{VideoGenerationRequest, VideoJobDeleteOutcome, VideoJobState, VideoJobStatus};
#[cfg(test)]
mod tests;
