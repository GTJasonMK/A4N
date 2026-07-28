use serde_json::Value;

use super::super::{ImportMediaPayload, MANIFEST_PATH, VideoStudioProjectPackage};
use super::payload::read_media_payloads_from_entries;

pub(in super::super) async fn read_video_studio_package_bytes(
    runtime_profile: crate::state::RuntimeProfile,
    capabilities: &a4n_capabilities::A4nCapabilities,
    bytes: Vec<u8>,
) -> Result<(VideoStudioProjectPackage, ImportMediaPayload), String> {
    let (manifest, media_bytes) = crate::sidecar_clients::read_import_media_package_zip(
        runtime_profile,
        capabilities,
        bytes,
        MANIFEST_PATH,
        "视频画布项目包",
        "视频画布项目清单",
    )
    .await?;
    let package = parse_video_studio_package_manifest(manifest)?;
    let media_payload = read_media_payloads_from_entries(&package, media_bytes)?;
    Ok((package, media_payload))
}

#[cfg(test)]
pub(super) fn read_video_studio_package_reader(
    bytes: Vec<u8>,
) -> Result<(VideoStudioProjectPackage, ImportMediaPayload), String> {
    let (manifest, media_bytes) = a4n_import_core::read_media_package_zip_bytes(
        &bytes,
        MANIFEST_PATH,
        "视频画布项目包",
        "视频画布项目清单",
    )
    .map_err(|err| err.to_string())?;
    let package = parse_video_studio_package_manifest(manifest)?;
    let media_payload = read_media_payloads_from_entries(&package, media_bytes)?;
    Ok((package, media_payload))
}

fn parse_video_studio_package_manifest(
    manifest: Value,
) -> Result<VideoStudioProjectPackage, String> {
    a4n_import_core::parse_json_manifest_value(manifest, "视频画布项目清单")
        .map_err(|err| err.to_string())
}
