use super::super::super::super::ImportMediaPayload;
use super::context::MediaManifestContext;

pub(super) fn validate_media_payload(
    media_payload: &ImportMediaPayload,
    context: &MediaManifestContext<'_>,
) -> Result<(), String> {
    for entry in media_payload.entries.values() {
        if !context.media_asset_ids.contains(&entry.media_asset_id) {
            return Err(format!(
                "媒体文件条目引用了不存在的媒体资产: {}",
                entry.media_asset_id
            ));
        }
    }
    Ok(())
}
