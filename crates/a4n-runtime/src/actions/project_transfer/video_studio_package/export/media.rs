use std::{collections::HashSet, path::Path};

use a4n_core::models::{
    ai::{generated_image, generated_video},
    media::media_asset,
};

use crate::{file_utils::canonicalize_data_file_path, state::AppState};

use super::super::{
    MEDIA_FIELD_FILE, MEDIA_FIELD_THUMBNAIL, MEDIA_OWNER_IMAGE, MEDIA_OWNER_MEDIA_ASSET,
    MEDIA_OWNER_VIDEO, VideoStudioMediaFileEntry,
};

struct OptionalMediaFiles<'a> {
    owner_kind: &'a str,
    owner_id: i64,
    file_path: Option<&'a str>,
    thumbnail_path: Option<&'a str>,
}

pub(super) fn collect_media_files(
    state: &AppState,
    images: &[generated_image::Model],
    videos: &[generated_video::Model],
    media_assets: &[media_asset::Model],
) -> Result<Vec<VideoStudioMediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for image in images {
        let package_path =
            media_package_path(MEDIA_OWNER_IMAGE, image.id, "file", &image.file_path)?;
        validate_export_media_file(
            state,
            &image.file_path,
            &package_path,
            &mut seen_package_paths,
        )?;
        entries.push(VideoStudioMediaFileEntry {
            owner_kind: MEDIA_OWNER_IMAGE.to_string(),
            owner_id: image.id,
            field: MEDIA_FIELD_FILE.to_string(),
            original_path: image.file_path.clone(),
            package_path,
        });
    }
    for video in videos {
        collect_optional_media_files(
            state,
            OptionalMediaFiles {
                owner_kind: MEDIA_OWNER_VIDEO,
                owner_id: video.id,
                file_path: video.file_path.as_deref(),
                thumbnail_path: video.thumbnail_path.as_deref(),
            },
            &mut entries,
            &mut seen_package_paths,
        )?;
    }
    for asset in media_assets {
        collect_optional_media_files(
            state,
            OptionalMediaFiles {
                owner_kind: MEDIA_OWNER_MEDIA_ASSET,
                owner_id: asset.id,
                file_path: asset.file_path.as_deref(),
                thumbnail_path: asset.thumbnail_path.as_deref(),
            },
            &mut entries,
            &mut seen_package_paths,
        )?;
    }
    Ok(entries)
}

fn collect_optional_media_files(
    state: &AppState,
    source: OptionalMediaFiles<'_>,
    entries: &mut Vec<VideoStudioMediaFileEntry>,
    seen_package_paths: &mut HashSet<String>,
) -> Result<(), String> {
    for (field, package_field, path) in [
        (MEDIA_FIELD_FILE, "file", source.file_path),
        (MEDIA_FIELD_THUMBNAIL, "thumbnail", source.thumbnail_path),
    ] {
        let Some(original_path) = path else {
            continue;
        };
        let package_path = media_package_path(
            source.owner_kind,
            source.owner_id,
            package_field,
            original_path,
        )?;
        validate_export_media_file(state, original_path, &package_path, seen_package_paths)?;
        entries.push(VideoStudioMediaFileEntry {
            owner_kind: source.owner_kind.to_string(),
            owner_id: source.owner_id,
            field: field.to_string(),
            original_path: original_path.to_string(),
            package_path,
        });
    }
    Ok(())
}

fn validate_export_media_file(
    state: &AppState,
    original_path: &str,
    package_path: &str,
    seen_package_paths: &mut HashSet<String>,
) -> Result<(), String> {
    canonicalize_data_file_path(
        &state.config.data_dir,
        original_path,
        "视频画布项目媒体文件",
    )?;
    if !seen_package_paths.insert(package_path.to_string()) {
        return Err(format!("视频画布项目媒体包路径重复: {package_path}"));
    }
    Ok(())
}

fn media_package_path(
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = Path::new(original_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {original_path}"))?;
    Ok(format!("media/{owner_kind}/{owner_id}/{field}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::media_package_path;

    #[test]
    fn media_package_path_preserves_owner_field_and_extension() {
        let path = media_package_path("generated_video", 42, "thumbnail", "poster.webp")
            .expect("media path should be valid");

        assert_eq!(path, "media/generated_video/42/thumbnail.webp");
    }

    #[test]
    fn media_package_path_rejects_missing_extension() {
        let err = media_package_path("media_asset", 7, "file", "projects/7/video")
            .expect_err("extension is required");

        assert_eq!(err, "媒体文件缺少扩展名: projects/7/video");
    }
}
