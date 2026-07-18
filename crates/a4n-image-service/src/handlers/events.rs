mod api;
mod execution;
mod projection;

pub(crate) use api::generate_events;

#[cfg(test)]
pub(crate) use a4n_sidecar_bootstrap::service_event_line as event_line;
#[cfg(test)]
pub(crate) use projection::{
    completed_generation_event, failed_generation_event, progress_generation_event,
};
