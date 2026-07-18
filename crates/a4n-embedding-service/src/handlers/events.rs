mod api;
mod execution;
mod projection;

pub(crate) use api::download_model_events;

#[cfg(test)]
pub(crate) use a4n_sidecar_bootstrap::service_event_line as event_line;
#[cfg(test)]
pub(crate) use projection::{
    completed_download_event, download_progress_event, failed_download_event,
};
