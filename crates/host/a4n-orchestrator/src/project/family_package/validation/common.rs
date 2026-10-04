use super::*;

pub(crate) fn validate_media_bytes(
    asset: &a4n_domain::project::ProjectFamilyMediaAssetRecord,
    bytes: &[u8],
) -> Result<(), String> {
    let expected_size = asset
        .file_size
        .ok_or_else(|| format!("媒体资产缺少 file_size: {}", asset.id))?;
    if expected_size != bytes.len() as i64 {
        return Err(format!(
            "媒体文件大小与清单不一致: media_asset_id={}, expected={}, actual={}",
            asset.id,
            expected_size,
            bytes.len()
        ));
    }
    let expected_checksum = asset
        .checksum
        .as_deref()
        .ok_or_else(|| format!("媒体资产缺少 checksum: {}", asset.id))?;
    let actual_checksum = sha256_bytes(bytes);
    if expected_checksum != actual_checksum {
        return Err(format!(
            "媒体文件 checksum 与清单不一致: media_asset_id={}",
            asset.id
        ));
    }
    Ok(())
}
