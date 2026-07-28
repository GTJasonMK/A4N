use crate::project::sha256_bytes;
use crate::project::video_studio_package::{
    MEDIA_FIELD_FILE, MEDIA_FIELD_THUMBNAIL, MEDIA_OWNER_IMAGE, MEDIA_OWNER_MEDIA_ASSET,
    MEDIA_OWNER_VIDEO, VideoStudioMediaFileEntry, VideoStudioProjectPackage,
};

pub(crate) fn validate_media_entry_bytes(
    package: &VideoStudioProjectPackage,
    entry: &VideoStudioMediaFileEntry,
    bytes: &[u8],
) -> Result<(), String> {
    match (entry.owner_kind.as_str(), entry.field.as_str()) {
        (MEDIA_OWNER_IMAGE, MEDIA_FIELD_FILE) => {
            let image = package
                .generated_images
                .iter()
                .find(|record| record.id == entry.owner_id)
                .ok_or_else(|| {
                    format!(
                        "媒体清单引用了不存在的生成图片: generated_image_id={}",
                        entry.owner_id
                    )
                })?;
            validate_payload_size(
                image.file_size,
                bytes,
                &entry.owner_kind,
                entry.owner_id,
                &entry.field,
            )
        }
        (MEDIA_OWNER_VIDEO, MEDIA_FIELD_FILE) => {
            let video = package
                .generated_videos
                .iter()
                .find(|record| record.id == entry.owner_id)
                .ok_or_else(|| {
                    format!(
                        "媒体清单引用了不存在的生成视频: generated_video_id={}",
                        entry.owner_id
                    )
                })?;
            validate_payload_size(
                video.file_size,
                bytes,
                &entry.owner_kind,
                entry.owner_id,
                &entry.field,
            )
        }
        (MEDIA_OWNER_MEDIA_ASSET, MEDIA_FIELD_FILE) => {
            let asset = package
                .media_assets
                .iter()
                .find(|record| record.id == entry.owner_id)
                .ok_or_else(|| {
                    format!(
                        "媒体清单引用了不存在的媒体资产: media_asset_id={}",
                        entry.owner_id
                    )
                })?;
            validate_payload_size(
                asset.file_size,
                bytes,
                &entry.owner_kind,
                entry.owner_id,
                &entry.field,
            )?;
            if let Some(expected_checksum) = asset.checksum.as_deref() {
                let actual_checksum = sha256_bytes(bytes);
                if actual_checksum != expected_checksum {
                    return Err(format!(
                        "媒体文件 checksum 与清单不一致: owner_kind={}, owner_id={}, field={}, expected={}, actual={}",
                        entry.owner_kind,
                        entry.owner_id,
                        entry.field,
                        expected_checksum,
                        actual_checksum
                    ));
                }
            }
            Ok(())
        }
        (MEDIA_OWNER_VIDEO, MEDIA_FIELD_THUMBNAIL)
        | (MEDIA_OWNER_MEDIA_ASSET, MEDIA_FIELD_THUMBNAIL) => Ok(()),
        _ => Err(format!(
            "媒体清单字段无效: owner_kind={}, field={}",
            entry.owner_kind, entry.field
        )),
    }
}

fn validate_payload_size(
    expected_size: Option<i64>,
    bytes: &[u8],
    owner_kind: &str,
    owner_id: i64,
    field: &str,
) -> Result<(), String> {
    let Some(expected_size) = expected_size else {
        return Ok(());
    };
    if expected_size != bytes.len() as i64 {
        return Err(format!(
            "媒体文件大小与清单不一致: owner_kind={owner_kind}, owner_id={owner_id}, field={field}, expected={expected_size}, actual={}",
            bytes.len()
        ));
    }
    Ok(())
}
