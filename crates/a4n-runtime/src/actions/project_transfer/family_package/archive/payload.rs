use std::collections::HashMap;

use super::super::validation::{validate_media_bytes, validate_package_entry_path};
use super::super::{ImportMediaPayload, ProjectFamilyPackage, media_asset_path, media_file_key};

pub(super) fn read_media_payloads_from_entries(
    package: &ProjectFamilyPackage,
    bytes: HashMap<String, Vec<u8>>,
) -> Result<ImportMediaPayload, String> {
    let media_assets = package
        .media_assets
        .iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    let mut entries = HashMap::new();
    for entry in &package.media_files {
        validate_package_entry_path(&entry.package_path)?;
        let asset = media_assets.get(&entry.media_asset_id).ok_or_else(|| {
            format!(
                "媒体文件清单引用了不存在的 media_asset_id: {}",
                entry.media_asset_id
            )
        })?;
        let expected_path = media_asset_path(asset, &entry.field)?;
        if expected_path.is_none() {
            return Err(format!(
                "媒体文件清单字段 {} 与媒体资产 {} 不匹配",
                entry.field, asset.id
            ));
        }
        let key = media_file_key(entry.media_asset_id, &entry.field);
        if entries.insert(key, entry.clone()).is_some() {
            return Err(format!(
                "媒体文件清单重复: media_asset_id={}, field={}",
                entry.media_asset_id, entry.field
            ));
        }
    }

    for entry in &package.media_files {
        let asset = media_assets.get(&entry.media_asset_id).ok_or_else(|| {
            format!(
                "媒体文件清单引用了不存在的 media_asset_id: {}",
                entry.media_asset_id
            )
        })?;
        let file_bytes = bytes
            .get(&entry.package_path)
            .ok_or_else(|| format!("项目族导入包缺少媒体文件 {}", entry.package_path))?;
        if entry.field == "file_path" {
            validate_media_bytes(asset, file_bytes)?;
        }
    }
    Ok(ImportMediaPayload { entries, bytes })
}
