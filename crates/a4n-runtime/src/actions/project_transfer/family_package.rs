//! 项目族包 Host 适配：sidecar ZIP、媒体物理 IO 与 application Host Port。

#[cfg(test)]
use a4n_core::models::{
    generation::generation_spec,
    project::{source_snapshot_outline, source_snapshot_profile},
    source_sync::source_sync_conflict,
};
use a4n_orchestrator::project::{
    FAMILY_PACKAGE_MANIFEST_PATH as MANIFEST_PATH, FamilyImportMediaPayload, ProjectFamilyPackage,
};
#[cfg(test)]
use a4n_orchestrator::project::{PROJECT_FAMILY_PACKAGE_VERSION, TextCorePackage, hash_json};
#[cfg(test)]
use a4n_orchestrator::source_sync::CONFLICT_STATUS_PENDING;
#[cfg(test)]
use std::collections::HashMap;

mod archive;
mod host_port;
mod package_io;

pub(in crate::actions::project_transfer) use host_port::RuntimeProjectFamilyTransferHostPort;

#[cfg(test)]
mod tests;
