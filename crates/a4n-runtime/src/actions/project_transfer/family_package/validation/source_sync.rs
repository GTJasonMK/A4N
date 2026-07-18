use super::*;

mod contracts;
mod decision;
mod json;
mod manifest;
mod references;
mod rules;
mod stages;

pub(super) fn validate_source_sync_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    manifest::validate_source_sync_manifest(package)
}
