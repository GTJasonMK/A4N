mod contracts;
mod events;
mod operations;

pub(crate) use operations::{
    delete_video_job, discover_video_models, generate_video, poll_video,
    preview_video_provider_request, probe_video_config, video_job_status,
};

#[cfg(test)]
use crate::state::RuntimeProfile;
#[cfg(test)]
use a4n_capabilities::A4nCapabilities;
#[cfg(test)]
use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::ai::video_gen_config;
#[cfg(test)]
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceErrorCode};
#[cfg(test)]
use a4n_video::{VideoGenerationRequest, VideoJobDeleteOutcome, VideoJobState, VideoJobStatus};
#[cfg(test)]
use events::*;

#[cfg(test)]
mod tests;
