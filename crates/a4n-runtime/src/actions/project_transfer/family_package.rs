//! 项目族包 ZIP、manifest 与媒体 payload 适配。

#[cfg(test)]
use a4n_core::models::{
    generation::generation_spec,
    project::{source_snapshot_outline, source_snapshot_profile},
    source_sync::source_sync_conflict,
};
#[cfg(test)]
use a4n_domain::source_sync::SourceSyncConflictStatus;
use a4n_orchestrator::project::{
    FAMILY_PACKAGE_MANIFEST_PATH as MANIFEST_PATH, FamilyImportMediaPayload, ProjectFamilyPackage,
};
#[cfg(test)]
use a4n_orchestrator::project::{PROJECT_FAMILY_PACKAGE_VERSION, TextCorePackage, hash_json};
#[cfg(test)]
use std::collections::HashMap;

mod archive;
mod package_io;

pub(in crate::actions::project_transfer) use archive::read_project_family_package_bytes;
pub(in crate::actions::project_transfer) use package_io::package_zip_bytes;

#[cfg(test)]
mod tests;
