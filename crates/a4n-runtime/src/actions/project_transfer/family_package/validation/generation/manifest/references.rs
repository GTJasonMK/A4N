mod context;
mod links;
mod specs;

use crate::actions::project_transfer::family_package::ProjectFamilyPackage;

use context::GenerationReferenceContext;

pub(super) fn validate_generation_reference_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let refs = GenerationReferenceContext::new(package);
    specs::validate_generation_specs(package, &refs)?;
    links::validate_generation_links(package, &refs)
}
