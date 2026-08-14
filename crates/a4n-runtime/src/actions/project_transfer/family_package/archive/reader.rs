use super::super::{FamilyImportMediaPayload, MANIFEST_PATH, ProjectFamilyPackage};
use super::payload::read_media_payloads_from_entries;

pub(in crate::actions::project_transfer) async fn read_project_family_package_bytes(
    runtime_profile: crate::state::RuntimeProfile,
    capabilities: &a4n_capabilities::A4nCapabilities,
    bytes: Vec<u8>,
) -> Result<(ProjectFamilyPackage, FamilyImportMediaPayload), String> {
    let (manifest, media_bytes) = crate::sidecar_clients::read_import_media_package_zip(
        runtime_profile,
        capabilities,
        bytes,
        MANIFEST_PATH,
        "项目族导入包",
        "项目族清单",
    )
    .await?;
    let package = a4n_import_core::parse_json_manifest_value(manifest, "项目族清单")
        .map_err(|err| err.to_string())?;
    let media_payload = read_media_payloads_from_entries(&package, media_bytes)?;
    Ok((package, media_payload))
}

#[cfg(test)]
pub(super) fn read_project_family_package_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<(ProjectFamilyPackage, FamilyImportMediaPayload), String> {
    let (manifest, media_bytes) = a4n_import_core::read_media_package_zip_reader(
        reader,
        MANIFEST_PATH,
        "项目族导入包",
        "项目族清单",
    )
    .map_err(|err| err.to_string())?;
    let package = a4n_import_core::parse_json_manifest_value(manifest, "项目族清单")
        .map_err(|err| err.to_string())?;
    let media_payload = read_media_payloads_from_entries(&package, media_bytes)?;
    Ok((package, media_payload))
}
