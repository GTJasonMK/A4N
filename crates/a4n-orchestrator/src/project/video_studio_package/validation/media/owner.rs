use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_core::models::{ai::generated_image, ai::generated_video, media::media_asset};
use a4n_domain::CanvasNodeRefKind;

use crate::project::video_studio_package::{VideoStudioMediaFileEntry, VideoStudioProjectPackage};

pub(super) enum MediaEntryOwner<'a> {
    GeneratedImage(&'a generated_image::Model),
    GeneratedVideo(&'a generated_video::Model),
    MediaAsset(&'a media_asset::Model),
}

pub(super) fn resolve_media_entry_owner<'a>(
    package: &'a VideoStudioProjectPackage,
    entry: &VideoStudioMediaFileEntry,
) -> Result<MediaEntryOwner<'a>, String> {
    match (
        entry.owner_kind.parse::<CanvasNodeRefKind>().ok(),
        entry.field.as_str(),
    ) {
        (Some(CanvasNodeRefKind::GeneratedImage), FILE_PATH_FIELD) => package
            .generated_images
            .iter()
            .find(|record| record.id == entry.owner_id)
            .map(MediaEntryOwner::GeneratedImage)
            .ok_or_else(|| {
                format!(
                    "媒体清单引用了不存在的生成图片: generated_image_id={}",
                    entry.owner_id
                )
            }),
        (Some(CanvasNodeRefKind::GeneratedVideo), FILE_PATH_FIELD | THUMBNAIL_PATH_FIELD) => package
            .generated_videos
            .iter()
            .find(|record| record.id == entry.owner_id)
            .map(MediaEntryOwner::GeneratedVideo)
            .ok_or_else(|| {
                format!(
                    "媒体清单引用了不存在的生成视频: generated_video_id={}",
                    entry.owner_id
                )
            }),
        (
            Some(CanvasNodeRefKind::MediaAsset),
            FILE_PATH_FIELD | THUMBNAIL_PATH_FIELD,
        ) => package
            .media_assets
            .iter()
            .find(|record| record.id == entry.owner_id)
            .map(MediaEntryOwner::MediaAsset)
            .ok_or_else(|| {
                format!(
                    "媒体清单引用了不存在的媒体资产: media_asset_id={}",
                    entry.owner_id
                )
            }),
        _ => Err(format!(
            "媒体清单字段无效: owner_kind={}, field={}",
            entry.owner_kind, entry.field
        )),
    }
}

pub(crate) fn validate_media_entry_owner(
    package: &VideoStudioProjectPackage,
    entry: &VideoStudioMediaFileEntry,
) -> Result<(), String> {
    match resolve_media_entry_owner(package, entry)? {
        MediaEntryOwner::GeneratedImage(image) => {
            if image.file_path != entry.original_path {
                return Err(format!(
                    "媒体清单图片路径与记录不一致: generated_image_id={}",
                    image.id
                ));
            }
        }
        MediaEntryOwner::GeneratedVideo(video) => {
            let expected = if entry.field == FILE_PATH_FIELD {
                video.file_path.as_deref()
            } else {
                video.thumbnail_path.as_deref()
            };
            if expected != Some(entry.original_path.as_str()) {
                let label = if entry.field == FILE_PATH_FIELD {
                    "视频路径"
                } else {
                    "视频缩略图路径"
                };
                return Err(format!(
                    "媒体清单{label}与记录不一致: generated_video_id={}",
                    video.id
                ));
            }
        }
        MediaEntryOwner::MediaAsset(asset) => {
            let expected = if entry.field == FILE_PATH_FIELD {
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
    }
    Ok(())
}
