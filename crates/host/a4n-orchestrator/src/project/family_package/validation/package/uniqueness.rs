mod ids;
mod keys;

use super::super::super::ProjectFamilyPackage;

pub(super) fn validate_package_uniqueness(package: &ProjectFamilyPackage) -> Result<(), String> {
    ids::validate_primary_row_ids(package)?;
    keys::validate_generation_ref_keys(package)?;
    ids::validate_secondary_row_ids(package)
}
