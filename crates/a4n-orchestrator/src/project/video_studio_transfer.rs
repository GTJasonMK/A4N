use std::collections::HashSet;

use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_db::canvas::VideoStudioProjectExportRows;
use a4n_domain::CanvasNodeRefKind;
use async_trait::async_trait;

use super::{
    ImportMediaPayload, MediaFileEntry, ProjectPackageFileHostPort, VideoStudioProjectPackage,
    required_path_extension,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportVideoStudioProjectApplicationInput {
    pub user_id: i64,
    pub project_id: i64,
}

#[derive(Debug)]
pub struct ReadVideoStudioProjectPackage {
    pub package: VideoStudioProjectPackage,
    pub media_payload: ImportMediaPayload,
}

#[async_trait]
pub trait VideoStudioProjectTransferHostPort: ProjectPackageFileHostPort {
    async fn render_export_package(
        &self,
        package: &VideoStudioProjectPackage,
    ) -> Result<Vec<u8>, String>;

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
    ) -> Result<ReadVideoStudioProjectPackage, String>;
}

pub(crate) fn video_studio_export_media_files(
    rows: &VideoStudioProjectExportRows,
) -> Result<Vec<MediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for image in &rows.generated_images {
        push_media_file(
            &mut entries,
            &mut seen_package_paths,
            CanvasNodeRefKind::GeneratedImage.as_str(),
            image.id,
            FILE_PATH_FIELD,
            "file",
            &image.file_path,
        )?;
    }
    for video in &rows.generated_videos {
        push_optional_media_files(
            &mut entries,
            &mut seen_package_paths,
            CanvasNodeRefKind::GeneratedVideo.as_str(),
            video.id,
            video.file_path.as_deref(),
            video.thumbnail_path.as_deref(),
        )?;
    }
    for asset in &rows.media_assets {
        push_optional_media_files(
            &mut entries,
            &mut seen_package_paths,
            CanvasNodeRefKind::MediaAsset.as_str(),
            asset.id,
            asset.file_path.as_deref(),
            asset.thumbnail_path.as_deref(),
        )?;
    }
    Ok(entries)
}

fn push_optional_media_files(
    entries: &mut Vec<MediaFileEntry>,
    seen_package_paths: &mut HashSet<String>,
    owner_kind: &str,
    owner_id: i64,
    file_path: Option<&str>,
    thumbnail_path: Option<&str>,
) -> Result<(), String> {
    for (field, package_field, path) in [
        (FILE_PATH_FIELD, "file", file_path),
        (THUMBNAIL_PATH_FIELD, "thumbnail", thumbnail_path),
    ] {
        let Some(original_path) = path else {
            continue;
        };
        push_media_file(
            entries,
            seen_package_paths,
            owner_kind,
            owner_id,
            field,
            package_field,
            original_path,
        )?;
    }
    Ok(())
}

fn push_media_file(
    entries: &mut Vec<MediaFileEntry>,
    seen_package_paths: &mut HashSet<String>,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    package_field: &str,
    original_path: &str,
) -> Result<(), String> {
    let package_path = media_package_path(owner_kind, owner_id, package_field, original_path)?;
    if !seen_package_paths.insert(package_path.clone()) {
        return Err(format!("视频画布项目媒体包路径重复: {package_path}"));
    }
    entries.push(MediaFileEntry {
        owner_kind: owner_kind.to_string(),
        owner_id,
        field: field.to_string(),
        original_path: original_path.to_string(),
        package_path,
    });
    Ok(())
}

fn media_package_path(
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = required_path_extension(original_path, "媒体文件")?;
    Ok(format!("media/{owner_kind}/{owner_id}/{field}.{extension}"))
}

#[cfg(test)]
mod tests;
