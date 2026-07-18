use serde_json::Value;

use crate::state::AppState;

use super::super::{ImportMediaPayload, MANIFEST_PATH, ProjectFamilyPackage};
use super::payload::read_media_payloads_from_entries;

pub(super) async fn read_project_family_package_bytes(
    state: &AppState,
    bytes: Vec<u8>,
) -> Result<(ProjectFamilyPackage, ImportMediaPayload), String> {
    let (manifest, media_bytes) = crate::sidecar_clients::read_import_media_package_zip(
        state.runtime_profile,
        state.capabilities(),
        bytes,
        MANIFEST_PATH,
        "项目族导入包",
        "项目族清单",
    )
    .await?;
    let package = parse_project_family_manifest(manifest)?;
    let media_payload = read_media_payloads_from_entries(&package, media_bytes)?;
    Ok((package, media_payload))
}

#[cfg(test)]
pub(super) fn read_project_family_package_reader<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<(ProjectFamilyPackage, ImportMediaPayload), String> {
    let (manifest, media_bytes) = a4n_import_core::read_media_package_zip_reader(
        reader,
        MANIFEST_PATH,
        "项目族导入包",
        "项目族清单",
    )
    .map_err(|err| err.to_string())?;
    let package = parse_project_family_manifest(manifest)?;
    let media_payload = read_media_payloads_from_entries(&package, media_bytes)?;
    Ok((package, media_payload))
}

fn parse_project_family_manifest(manifest: Value) -> Result<ProjectFamilyPackage, String> {
    a4n_import_core::parse_json_manifest_value(manifest, "项目族清单")
        .map_err(|err| err.to_string())
}
