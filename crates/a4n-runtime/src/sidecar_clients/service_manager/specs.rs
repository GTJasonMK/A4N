mod catalog;
mod environment;

pub(super) use catalog::{
    MANAGED_SIDECARS_ENV, SIDECAR_BIN_DIR_ENV, SidecarServiceSpec, capability_service_names,
    sidecar_service_specs,
};
pub(in crate::sidecar_clients) use environment::sidecar_url_for_service;
pub(super) use environment::{managed_sidecars_enabled, sidecar_addr, sidecar_url_from_spec};
#[cfg(test)]
pub(super) use environment::{
    parse_managed_sidecars_env, parse_sidecar_addr_env, parse_sidecar_url_env,
};
