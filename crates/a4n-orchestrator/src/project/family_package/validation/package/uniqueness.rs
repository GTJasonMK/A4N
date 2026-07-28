mod ids;
mod keys;

use super::super::super::ProjectFamilyPackage;
use super::super::common::validate_unique_ids;

pub(super) fn validate_package_uniqueness(package: &ProjectFamilyPackage) -> Result<(), String> {
    ids::validate_primary_row_ids(package)?;
    keys::validate_generation_ref_keys(package)?;
    ids::validate_secondary_row_ids(package)?;
    validate_unique_ids(
        package
            .project_derivations
            .iter()
            .map(|record| record.derived_project_id),
        "project_derivation.derived_project_id",
    )
}
