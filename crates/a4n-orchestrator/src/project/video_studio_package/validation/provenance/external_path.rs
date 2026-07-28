use super::super::*;

pub(in crate::project::video_studio_package::validation) fn validate_media_provenance_source_external_path(
    provenance: &media_provenance::Model,
    image_ids: &HashSet<i64>,
    video_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(raw) = provenance.source_external_path.as_deref() else {
        return Ok(());
    };
    if let Some(suffix) = raw.strip_prefix("generated_video:") {
        let video_id = suffix.parse::<i64>().map_err(|_| {
            format!(
                "媒体来源 source_external_path 的 generated_video ID 无效: media_asset_id={}, source_external_path={raw}",
                provenance.media_asset_id
            )
        })?;
        if !video_ids.contains(&video_id) {
            return Err(format!(
                "媒体来源 source_external_path 引用了不存在的生成视频: media_asset_id={}, generated_video_id={video_id}",
                provenance.media_asset_id
            ));
        }
    } else if let Some(suffix) = raw.strip_prefix("generated_image:") {
        let image_id = suffix.parse::<i64>().map_err(|_| {
            format!(
                "媒体来源 source_external_path 的 generated_image ID 无效: media_asset_id={}, source_external_path={raw}",
                provenance.media_asset_id
            )
        })?;
        if !image_ids.contains(&image_id) {
            return Err(format!(
                "媒体来源 source_external_path 引用了不存在的生成图片: media_asset_id={}, generated_image_id={image_id}",
                provenance.media_asset_id
            ));
        }
    }
    Ok(())
}
