pub(in crate::sidecar_clients::service_manager) use a4n_service_protocol::{
    MANAGED_SIDECARS_ENV, SIDECAR_BIN_DIR_ENV,
};
use a4n_service_protocol::{ServiceCatalogEntry, service_catalog};

pub(in crate::sidecar_clients::service_manager) fn sidecar_service_specs()
-> Vec<ServiceCatalogEntry> {
    service_catalog().expect("embedded sidecar service catalog must be valid")
}

pub(in crate::sidecar_clients::service_manager) fn capability_service_names() -> Vec<String> {
    sidecar_service_specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect()
}
