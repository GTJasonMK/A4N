use std::collections::HashMap;

use super::super::ProjectFamilyPackage;
use a4n_orchestrator::project::{ImportMediaPayload, media_file_key};

pub(super) fn read_media_payloads_from_entries(
    package: &ProjectFamilyPackage,
    bytes: HashMap<String, Vec<u8>>,
) -> Result<ImportMediaPayload, String> {
    let mut entries = HashMap::new();
    for entry in &package.media_files {
        let key = media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
        if entries.insert(key, entry.clone()).is_some() {
            return Err(format!(
                "媒体文件清单重复: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
    }

    for entry in &package.media_files {
        bytes
            .get(&entry.package_path)
            .ok_or_else(|| format!("项目族导入包缺少媒体文件 {}", entry.package_path))?;
    }
    Ok(ImportMediaPayload { entries, bytes })
}
