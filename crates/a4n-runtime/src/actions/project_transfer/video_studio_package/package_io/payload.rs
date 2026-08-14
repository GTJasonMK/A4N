use std::collections::HashMap;

use super::super::{ImportMediaPayload, VideoStudioProjectPackage};
use a4n_orchestrator::project::{
    validate_media_entry_bytes, validate_media_entry_owner, video_studio_media_file_key,
};

pub(super) fn read_media_payloads_from_entries(
    package: &VideoStudioProjectPackage,
    bytes: HashMap<String, Vec<u8>>,
) -> Result<ImportMediaPayload, String> {
    let mut entries = HashMap::new();
    for entry in &package.media_files {
        validate_media_entry_owner(package, entry)?;
        let key = video_studio_media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
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
        validate_media_entry_bytes(package, entry, file_bytes)?;
    }
    Ok(ImportMediaPayload { entries, bytes })
}
