use super::*;

pub fn snapshot_profile_key(kind: &str, id: i64) -> String {
    let snapshot_kind = match kind {
        "character_profile" => "character",
        value => value,
    };
    format!("{snapshot_kind}/{id}")
}

pub fn validate_unique_key_strings(
    keys: impl Iterator<Item = String>,
    label: &str,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for key in keys {
        if !seen.insert(key.clone()) {
            return Err(format!("导入包包含重复 {label}: {key}"));
        }
    }
    Ok(())
}

pub(super) fn validate_unique_ids(
    ids: impl Iterator<Item = i64>,
    label: &str,
) -> Result<(), String> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(format!("导入包包含重复 {label}: {id}"));
        }
    }
    Ok(())
}

pub fn validate_media_bytes(asset: &media_asset::Model, bytes: &[u8]) -> Result<(), String> {
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
