use std::collections::HashMap;

use super::super::validation;
use super::super::{ImportMediaPayload, VideoStudioProjectPackage, media_file_key};

pub(super) fn read_media_payloads_from_entries(
    package: &VideoStudioProjectPackage,
    bytes: HashMap<String, Vec<u8>>,
) -> Result<ImportMediaPayload, String> {
    let mut entries = HashMap::new();
    for entry in &package.media_files {
        validate_package_entry_path(&entry.package_path)?;
        validation::validate_media_entry_owner(package, entry)?;
        let key = media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
        if entries.insert(key, entry.clone()).is_some() {
            return Err(format!(
                "视频画布项目媒体清单重复: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
    }

    for entry in &package.media_files {
        let file_bytes = bytes
            .get(&entry.package_path)
            .ok_or_else(|| format!("视频画布项目包缺少媒体文件 {}", entry.package_path))?;
        if file_bytes.is_empty() {
            return Err(format!("媒体文件不能为空: {}", entry.package_path));
        }
        validation::validate_media_entry_bytes(package, entry, file_bytes)?;
    }
    Ok(ImportMediaPayload { entries, bytes })
}

fn validate_package_entry_path(path: &str) -> Result<(), String> {
    a4n_import_core::validate_zip_entry_path(path).map_err(|err| err.to_string())
}
