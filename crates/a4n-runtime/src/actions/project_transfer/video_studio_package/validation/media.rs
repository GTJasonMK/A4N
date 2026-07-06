use super::*;
use sha2::{Digest, Sha256};

pub(super) fn validate_optional_file_size(
    file_size: Option<i64>,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    if let Some(file_size) = file_size
        && file_size < 0
    {
        return Err(format!(
            "媒体记录 file_size 不能为负数: owner_kind={owner_kind}, owner_id={owner_id}, file_size={file_size}"
        ));
    }
    Ok(())
}

pub(in crate::actions::project_transfer::video_studio_package) fn validate_media_entry_owner(
    package: &VideoStudioProjectPackage,
    entry: &VideoStudioMediaFileEntry,
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
            if image.file_path != entry.original_path {
                return Err(format!(
                    "媒体清单图片路径与记录不一致: generated_image_id={}",
                    image.id
                ));
            }
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
            if video.file_path.as_deref() != Some(entry.original_path.as_str()) {
                return Err(format!(
                    "媒体清单视频路径与记录不一致: generated_video_id={}",
                    video.id
                ));
            }
        }
        (MEDIA_OWNER_VIDEO, MEDIA_FIELD_THUMBNAIL) => {
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
            if video.thumbnail_path.as_deref() != Some(entry.original_path.as_str()) {
                return Err(format!(
                    "媒体清单视频缩略图路径与记录不一致: generated_video_id={}",
                    video.id
                ));
            }
        }
        (MEDIA_OWNER_MEDIA_ASSET, MEDIA_FIELD_FILE | MEDIA_FIELD_THUMBNAIL) => {
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
            let expected = if entry.field == MEDIA_FIELD_FILE {
                asset.file_path.as_deref()
            } else {
                asset.thumbnail_path.as_deref()
            };
            if expected != Some(entry.original_path.as_str()) {
                return Err(format!(
                    "媒体清单媒体资产路径与记录不一致: media_asset_id={}, field={}",
                    asset.id, entry.field
                ));
            }
        }
        _ => {
            return Err(format!(
                "媒体清单字段无效: owner_kind={}, field={}",
                entry.owner_kind, entry.field
            ));
        }
    }
    Ok(())
}

pub(in crate::actions::project_transfer::video_studio_package) fn validate_media_entry_bytes(
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

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(super) fn require_media_payload(
    media_payload: &ImportMediaPayload,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
) -> Result<(), String> {
    let key = media_file_key(owner_kind, owner_id, field);
    if !media_payload.entries.contains_key(&key) {
        return Err(format!(
            "视频画布项目包缺少媒体文件: owner_kind={owner_kind}, owner_id={owner_id}, field={field}"
        ));
    }
    Ok(())
}
