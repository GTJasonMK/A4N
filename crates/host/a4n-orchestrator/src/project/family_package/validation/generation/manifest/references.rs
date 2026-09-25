mod context;
mod links;
mod specs;

use crate::project::family_package::ProjectFamilyPackage;

use context::GenerationReferenceContext;

pub(super) fn validate_generation_reference_manifest(
    package: &ProjectFamilyPackage,
    validated_canvas_specs: &std::collections::HashSet<i64>,
) -> Result<(), String> {
    let refs = GenerationReferenceContext::new(package);
    specs::validate_generation_specs(package, &refs, validated_canvas_specs)?;
    links::validate_generation_links(package, &refs)
}
