use std::sync::OnceLock;

use crate::contracts::{ProcessMetadata, ServiceCatalogEntry, StatefulWorkerCatalogEntry};

pub fn service_catalog() -> Result<Vec<ServiceCatalogEntry>, serde_json::Error> {
    serde_json::from_str(include_str!("../service-catalog.json"))
}

pub fn stateful_worker_catalog() -> Result<Vec<StatefulWorkerCatalogEntry>, serde_json::Error> {
    serde_json::from_str(include_str!("../worker-catalog.json"))
}

fn embedded_service_catalog() -> &'static [ServiceCatalogEntry] {
    static CATALOG: OnceLock<Vec<ServiceCatalogEntry>> = OnceLock::new();
    CATALOG
        .get_or_init(|| service_catalog().expect("embedded service catalog must be valid"))
        .as_slice()
}

pub fn service_catalog_entry_ref(name: &str) -> Option<&'static ServiceCatalogEntry> {
    embedded_service_catalog()
        .iter()
        .find(|entry| entry.name == name)
}

pub fn service_catalog_entry_for_capability_version_ref(
    capability_version: &str,
) -> Option<&'static ServiceCatalogEntry> {
    embedded_service_catalog()
        .iter()
        .find(|entry| entry.capability_version == capability_version)
}

fn embedded_stateful_worker_catalog() -> &'static [StatefulWorkerCatalogEntry] {
    static CATALOG: OnceLock<Vec<StatefulWorkerCatalogEntry>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            stateful_worker_catalog().expect("embedded stateful worker catalog must be valid")
        })
        .as_slice()
}

pub fn process_catalog() -> Result<Vec<ProcessMetadata>, serde_json::Error> {
    let sidecars = service_catalog()?
        .iter()
        .map(ProcessMetadata::from)
        .collect::<Vec<_>>();
    let workers = stateful_worker_catalog()?
        .iter()
        .map(ProcessMetadata::from)
        .collect::<Vec<_>>();
    Ok(sidecars.into_iter().chain(workers).collect())
}

fn embedded_process_catalog() -> &'static [ProcessMetadata] {
    static CATALOG: OnceLock<Vec<ProcessMetadata>> = OnceLock::new();
    CATALOG
        .get_or_init(|| process_catalog().expect("embedded process catalog must be valid"))
        .as_slice()
}

pub fn process_catalog_entry(name: &str) -> Option<ProcessMetadata> {
    process_catalog_entry_ref(name).cloned()
}

fn process_catalog_entry_ref(name: &str) -> Option<&'static ProcessMetadata> {
    embedded_process_catalog()
        .iter()
        .find(|entry| entry.name == name)
}

pub fn stateful_worker_catalog_entry_ref(
    name: &str,
) -> Option<&'static StatefulWorkerCatalogEntry> {
    embedded_stateful_worker_catalog()
        .iter()
        .find(|entry| entry.name == name)
}
