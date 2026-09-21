mod references;
mod shape;

use crate::project::family_package::ProjectFamilyPackage;

pub(in crate::project::family_package::validation) fn validate_generation_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    shape::validate_generation_shape_manifest(package)?;
    references::validate_generation_reference_manifest(package)
}
