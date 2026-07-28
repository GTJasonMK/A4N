use super::super::*;
use super::contracts::ManifestIds;

pub(super) fn validate_payload_entries(
    media_payload: &ImportMediaPayload,
    ids: &ManifestIds,
) -> Result<(), String> {
    for entry in media_payload.entries.values() {
        match entry.owner_kind.as_str() {
            MEDIA_OWNER_IMAGE if !ids.image_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的生成图片: {}",
                    entry.owner_id
                ));
            }
            MEDIA_OWNER_VIDEO if !ids.video_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的生成视频: {}",
                    entry.owner_id
                ));
            }
            MEDIA_OWNER_MEDIA_ASSET if !ids.media_asset_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的媒体资产: {}",
                    entry.owner_id
                ));
            }
            MEDIA_OWNER_IMAGE | MEDIA_OWNER_VIDEO | MEDIA_OWNER_MEDIA_ASSET => {}
            other => return Err(format!("媒体文件条目 owner_kind 无效: {other}")),
        }
    }
    Ok(())
}
