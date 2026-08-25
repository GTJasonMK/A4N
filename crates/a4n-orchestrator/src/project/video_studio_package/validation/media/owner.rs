use std::collections::HashMap;

use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_core::models::{ai::generated_image, ai::generated_video, media::media_asset};
use a4n_domain::CanvasNodeRefKind;

use crate::project::{MediaFileEntry, VideoStudioProjectPackage};

pub(super) enum MediaEntryOwner<'a> {
    GeneratedImage(&'a generated_image::Model),
    GeneratedVideo(&'a generated_video::Model),
    MediaAsset(&'a media_asset::Model),
}

pub struct MediaEntryOwnerIndex<'a> {
    generated_images: HashMap<i64, &'a generated_image::Model>,
    generated_videos: HashMap<i64, &'a generated_video::Model>,
    media_assets: HashMap<i64, &'a media_asset::Model>,
}

impl<'a> MediaEntryOwnerIndex<'a> {
    pub fn new(package: &'a VideoStudioProjectPackage) -> Self {
        let mut generated_images = HashMap::with_capacity(package.generated_images.len());
        for record in &package.generated_images {
            generated_images.entry(record.id).or_insert(record);
        }
        let mut generated_videos = HashMap::with_capacity(package.generated_videos.len());
        for record in &package.generated_videos {
            generated_videos.entry(record.id).or_insert(record);
        }
        let mut media_assets = HashMap::with_capacity(package.media_assets.len());
        for record in &package.media_assets {
            media_assets.entry(record.id).or_insert(record);
        }
        Self {
            generated_images,
            generated_videos,
            media_assets,
        }
    }

    pub(super) fn resolve(&self, entry: &MediaFileEntry) -> Result<MediaEntryOwner<'a>, String> {
        match (
            entry.owner_kind.parse::<CanvasNodeRefKind>().ok(),
            entry.field.as_str(),
        ) {
            (Some(CanvasNodeRefKind::GeneratedImage), FILE_PATH_FIELD) => self
                .generated_images
                .get(&entry.owner_id)
                .copied()
                .map(MediaEntryOwner::GeneratedImage)
                .ok_or_else(|| {
                    format!(
                        "媒体清单引用了不存在的生成图片: generated_image_id={}",
                        entry.owner_id
                    )
                }),
            (Some(CanvasNodeRefKind::GeneratedVideo), FILE_PATH_FIELD | THUMBNAIL_PATH_FIELD) => {
                self.generated_videos
                    .get(&entry.owner_id)
                    .copied()
                    .map(MediaEntryOwner::GeneratedVideo)
                    .ok_or_else(|| {
                        format!(
                            "媒体清单引用了不存在的生成视频: generated_video_id={}",
                            entry.owner_id
                        )
                    })
            }
            (Some(CanvasNodeRefKind::MediaAsset), FILE_PATH_FIELD | THUMBNAIL_PATH_FIELD) => self
                .media_assets
                .get(&entry.owner_id)
                .copied()
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

    pub fn validate_owner(&self, entry: &MediaFileEntry) -> Result<(), String> {
        match self.resolve(entry)? {
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
}

pub(in crate::project::video_studio_package) fn validate_media_entry_owner(
    package: &VideoStudioProjectPackage,
    entry: &MediaFileEntry,
) -> Result<(), String> {
    MediaEntryOwnerIndex::new(package).validate_owner(entry)
}
