use std::collections::{HashMap, HashSet};

use super::super::super::common::validate_media_bytes;
use super::context::MediaManifestContext;
use crate::project::{
    ImportMediaPayload, family_media_package_path, family_package::ProjectFamilyPackage,
    media_asset_path, media_file_key,
};
use a4n_db_models::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};

const MEDIA_ASSET_OWNER: &str = "media_asset";
const GENERATED_IMAGE_OWNER: &str = "generated_image";
const GENERATED_VIDEO_OWNER: &str = "generated_video";
const CHARACTER_PORTRAIT_OWNER: &str = "character_portrait";

pub(super) fn validate_media_payload(
    package: &ProjectFamilyPackage,
    media_payload: &ImportMediaPayload,
    context: &MediaManifestContext<'_>,
) -> Result<(), String> {
    let images = package
        .generated_images
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let portraits = package
        .character_portraits
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let videos = package
        .generated_videos
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let mut manifest_keys = HashSet::new();
    let mut package_paths = HashSet::new();
    for entry in &package.media_files {
        let key = media_file_key(&entry.owner_kind, entry.owner_id, &entry.field);
        if !manifest_keys.insert(key.clone()) {
            return Err(format!(
                "媒体文件清单 owner/field 重复: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
        let payload_entry = media_payload.entries.get(&key).ok_or_else(|| {
            format!(
                "媒体文件清单缺少 payload 索引: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            )
        })?;
        if payload_entry.owner_kind != entry.owner_kind
            || payload_entry.owner_id != entry.owner_id
            || payload_entry.field != entry.field
            || payload_entry.original_path != entry.original_path
            || payload_entry.package_path != entry.package_path
        {
            return Err(format!(
                "媒体文件清单与 payload 索引不一致: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
        let expected_path = match entry.owner_kind.as_str() {
            MEDIA_ASSET_OWNER => {
                let asset = context
                    .media_assets_by_id
                    .get(&entry.owner_id)
                    .copied()
                    .ok_or_else(|| {
                        format!("媒体文件条目引用了不存在的媒体资产: {}", entry.owner_id)
                    })?;
                media_asset_path(asset, &entry.field)?.ok_or_else(|| {
                    format!(
                        "媒体文件条目字段与媒体资产不匹配: owner_id={}, field={}",
                        entry.owner_id, entry.field
                    )
                })?
            }
            GENERATED_IMAGE_OWNER => {
                if entry.field != "file_path" {
                    return Err(format!(
                        "生成图片媒体文件字段无效: owner_id={}, field={}",
                        entry.owner_id, entry.field
                    ));
                }
                images
                    .get(&entry.owner_id)
                    .map(|row| row.file_path.as_str())
                    .ok_or_else(|| {
                        format!("媒体文件条目引用了不存在的生成图片: {}", entry.owner_id)
                    })?
            }
            GENERATED_VIDEO_OWNER => {
                let video = videos.get(&entry.owner_id).copied().ok_or_else(|| {
                    format!("媒体文件条目引用了不存在的生成视频: {}", entry.owner_id)
                })?;
                match entry.field.as_str() {
                    FILE_PATH_FIELD => video.file_path.as_deref(),
                    THUMBNAIL_PATH_FIELD => video.thumbnail_path.as_deref(),
                    _ => {
                        return Err(format!(
                            "生成视频媒体文件字段无效: owner_id={}, field={}",
                            entry.owner_id, entry.field
                        ));
                    }
                }
                .ok_or_else(|| {
                    format!(
                        "生成视频媒体文件字段缺少路径: owner_id={}, field={}",
                        entry.owner_id, entry.field
                    )
                })?
            }
            CHARACTER_PORTRAIT_OWNER => {
                if entry.field != "image_path" {
                    return Err(format!(
                        "角色立绘媒体文件字段无效: owner_id={}, field={}",
                        entry.owner_id, entry.field
                    ));
                }
                portraits
                    .get(&entry.owner_id)
                    .map(|row| row.image_path.as_str())
                    .ok_or_else(|| {
                        format!("媒体文件条目引用了不存在的角色立绘: {}", entry.owner_id)
                    })?
            }
            other => return Err(format!("媒体文件 owner_kind 无效: {other}")),
        };
        if expected_path != entry.original_path {
            return Err(format!(
                "媒体文件条目原路径与 owner 不一致: owner_kind={}, owner_id={}, field={}",
                entry.owner_kind, entry.owner_id, entry.field
            ));
        }
        let expected_package_path = family_media_package_path(
            &entry.owner_kind,
            entry.owner_id,
            &entry.field,
            expected_path,
        )?;
        if entry.package_path != expected_package_path {
            return Err(format!(
                "媒体文件 package_path 非规范格式: owner_kind={}, owner_id={}, field={}, expected={expected_package_path}, actual={}",
                entry.owner_kind, entry.owner_id, entry.field, entry.package_path
            ));
        }
        if !package_paths.insert(&entry.package_path) {
            return Err(format!(
                "媒体文件 package_path 重复: {}",
                entry.package_path
            ));
        }
        let bytes = media_payload
            .bytes
            .get(&entry.package_path)
            .ok_or_else(|| format!("项目族导入包缺少媒体文件字节: {}", entry.package_path))?;
        if bytes.is_empty() {
            return Err(format!("项目族导入包媒体文件为空: {}", entry.package_path));
        }
        if entry.owner_kind == MEDIA_ASSET_OWNER && entry.field == FILE_PATH_FIELD {
            validate_media_bytes(context.media_assets_by_id[&entry.owner_id], bytes)?;
        }
        if entry.owner_kind == GENERATED_IMAGE_OWNER
            && let Some(expected_size) = images[&entry.owner_id].file_size
            && expected_size != bytes.len() as i64
        {
            return Err(format!(
                "生成图片文件大小与清单不一致: generated_image_id={}, expected={}, actual={}",
                entry.owner_id,
                expected_size,
                bytes.len()
            ));
        }
        if entry.owner_kind == GENERATED_VIDEO_OWNER
            && entry.field == FILE_PATH_FIELD
            && let Some(expected_size) = videos[&entry.owner_id].file_size
            && expected_size != bytes.len() as i64
        {
            return Err(format!(
                "生成视频文件大小与清单不一致: generated_video_id={}, expected={}, actual={}",
                entry.owner_id,
                expected_size,
                bytes.len()
            ));
        }
    }
    if media_payload.entries.len() != package.media_files.len() {
        return Err("媒体文件 payload 包含清单外索引".to_string());
    }
    if media_payload.bytes.len() != package_paths.len()
        || media_payload
            .bytes
            .keys()
            .any(|path| !package_paths.contains(path))
    {
        return Err("媒体文件 payload 包含清单外字节".to_string());
    }
    for image in &package.generated_images {
        require_owner_entry(media_payload, GENERATED_IMAGE_OWNER, image.id, "file_path")?;
    }
    for portrait in &package.character_portraits {
        require_owner_entry(
            media_payload,
            CHARACTER_PORTRAIT_OWNER,
            portrait.id,
            "image_path",
        )?;
    }
    for video in &package.generated_videos {
        if video.file_path.is_some() {
            require_owner_entry(
                media_payload,
                GENERATED_VIDEO_OWNER,
                video.id,
                FILE_PATH_FIELD,
            )?;
        }
        if video.thumbnail_path.is_some() {
            require_owner_entry(
                media_payload,
                GENERATED_VIDEO_OWNER,
                video.id,
                THUMBNAIL_PATH_FIELD,
            )?;
        }
    }
    Ok(())
}

fn require_owner_entry(
    media_payload: &ImportMediaPayload,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
) -> Result<(), String> {
    if !media_payload
        .entries
        .contains_key(&media_file_key(owner_kind, owner_id, field))
    {
        return Err(format!(
            "媒体 owner 缺少文件包条目: owner_kind={owner_kind}, owner_id={owner_id}, field={field}"
        ));
    }
    Ok(())
}
