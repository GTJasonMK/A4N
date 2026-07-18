mod assets;
mod bindings;
mod context;
mod payload;
mod provenance;

use super::super::super::{ImportMediaPayload, ProjectFamilyPackage};
use context::MediaManifestContext;

pub(in crate::actions::project_transfer::family_package) fn validate_media_manifest(
    package: &ProjectFamilyPackage,
    media_payload: &ImportMediaPayload,
) -> Result<(), String> {
    let context = MediaManifestContext::new(package);
    assets::validate_media_assets(package, media_payload, &context)?;
    provenance::validate_media_provenance(package, &context)?;
    bindings::validate_media_bindings(package, &context)?;
    payload::validate_media_payload(media_payload, &context)?;
    Ok(())
}
