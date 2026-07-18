mod contracts;
mod events;
mod operations;

pub(crate) use operations::{
    discover_video_models, generate_video, poll_video, preview_video_provider_request,
    probe_video_config,
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
use a4n_service_protocol::{ServiceErrorCode, ServiceTaskEvent, ServiceTaskEventKind};
#[cfg(test)]
use a4n_video::{VideoGenerationRequest, VideoJobState};
#[cfg(test)]
use events::*;

#[cfg(test)]
mod tests;
