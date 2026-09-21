mod catalog;
mod environment;

pub(super) use catalog::{
    MANAGED_SIDECARS_ENV, SIDECAR_BIN_DIR_ENV, capability_service_names, sidecar_service_specs,
};
pub(in crate::sidecar_clients) use environment::sidecar_service_and_url_for_capability;
pub(super) use environment::{
    managed_sidecars_enabled, parse_sidecar_addr_env, sidecar_url_from_spec,
};
