use a4n_service_protocol::{ServiceCatalogEntry, service_catalog};

pub(in crate::sidecar_clients::service_manager) const SIDECAR_BIN_DIR_ENV: &str =
    "A4N_SIDECAR_BIN_DIR";
pub(in crate::sidecar_clients::service_manager) const MANAGED_SIDECARS_ENV: &str =
    "A4N_MANAGED_SIDECARS";

pub(in crate::sidecar_clients::service_manager) type SidecarServiceSpec = ServiceCatalogEntry;

pub(in crate::sidecar_clients::service_manager) fn sidecar_service_specs() -> Vec<SidecarServiceSpec>
{
    service_catalog().expect("embedded sidecar service catalog must be valid")
}

pub(in crate::sidecar_clients::service_manager) fn capability_service_names() -> Vec<String> {
    sidecar_service_specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect()
}
