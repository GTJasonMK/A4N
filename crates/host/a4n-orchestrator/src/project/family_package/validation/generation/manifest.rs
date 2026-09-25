mod references;
mod shape;

use crate::project::family_package::ProjectFamilyPackage;

pub(in crate::project::family_package::validation) fn validate_generation_manifest(
    package: &ProjectFamilyPackage,
    validated_canvas_specs: &std::collections::HashSet<i64>,
) -> Result<(), String> {
    shape::validate_generation_shape_manifest(package)?;
    references::validate_generation_reference_manifest(package, validated_canvas_specs)
}
