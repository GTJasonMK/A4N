use super::contracts::ManifestIds;

use a4n_domain::CanvasNodeRefKind;

use crate::project::ImportMediaPayload;

pub(super) fn validate_payload_entries(
    media_payload: &ImportMediaPayload,
    ids: &ManifestIds,
) -> Result<(), String> {
    for entry in media_payload.entries.values() {
        match entry.owner_kind.parse::<CanvasNodeRefKind>() {
            Ok(CanvasNodeRefKind::GeneratedImage) if !ids.image_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的生成图片: {}",
                    entry.owner_id
                ));
            }
            Ok(CanvasNodeRefKind::GeneratedVideo) if !ids.video_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的生成视频: {}",
                    entry.owner_id
                ));
            }
            Ok(CanvasNodeRefKind::MediaAsset) if !ids.media_asset_ids.contains(&entry.owner_id) => {
                return Err(format!(
                    "媒体文件条目引用了不存在的媒体资产: {}",
                    entry.owner_id
                ));
            }
            Ok(
                CanvasNodeRefKind::GeneratedImage
                | CanvasNodeRefKind::GeneratedVideo
                | CanvasNodeRefKind::MediaAsset,
            ) => {}
            _ => {
                return Err(format!(
                    "媒体文件条目 owner_kind 无效: {}",
                    entry.owner_kind
                ));
            }
        }
    }
    Ok(())
}
