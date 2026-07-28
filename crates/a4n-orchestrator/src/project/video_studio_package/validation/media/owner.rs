use crate::project::video_studio_package::{
    MEDIA_FIELD_FILE, MEDIA_FIELD_THUMBNAIL, MEDIA_OWNER_IMAGE, MEDIA_OWNER_MEDIA_ASSET,
    MEDIA_OWNER_VIDEO, VideoStudioMediaFileEntry, VideoStudioProjectPackage,
};

pub(crate) fn validate_media_entry_owner(
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
