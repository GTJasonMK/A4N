mod binaries;
mod capabilities;
mod configs;
mod environment;
mod specs;

pub(crate) use capabilities::runtime_capabilities;
pub(crate) use configs::{runtime_service_manager, start_managed_sidecars};
pub(super) use specs::sidecar_service_and_url_for_capability;

#[cfg(test)]
mod tests;
