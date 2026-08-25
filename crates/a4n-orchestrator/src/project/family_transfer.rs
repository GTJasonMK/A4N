use std::collections::HashSet;

use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_db::project::ProjectFamilyExportRows;
use a4n_domain::CanvasNodeRefKind;
use async_trait::async_trait;

use super::{
    ImportMediaPayload, MediaFileEntry, ProjectFamilyPackage, ProjectPackageFileHostPort,
    family_media_package_path, media_asset_path, required_path_extension,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportProjectFamilyPackageApplicationInput {
    pub user_id: i64,
    pub root_project_id: i64,
    pub include_derived_projects: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportDerivedProjectPackageApplicationInput {
    pub user_id: i64,
    pub derived_project_id: i64,
}

pub struct ReadProjectFamilyPackage {
    pub package: ProjectFamilyPackage,
    pub media_payload: ImportMediaPayload,
}

#[async_trait]
pub trait ProjectFamilyTransferHostPort: ProjectPackageFileHostPort {
    async fn render_export_package(
        &self,
        package: &ProjectFamilyPackage,
    ) -> Result<Vec<u8>, String>;

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
    ) -> Result<ReadProjectFamilyPackage, String>;
}

pub(crate) fn project_family_export_media_files(
    rows: &ProjectFamilyExportRows,
) -> Result<Vec<MediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for asset in &rows.media_assets {
        for field in [FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD] {
            let Some(original_path) = media_asset_path(asset, field)? else {
                continue;
            };
            push_media_file(
                &mut entries,
                &mut seen_package_paths,
                "media_asset",
                asset.id,
                field,
                original_path,
            )?;
        }
    }
    for image in &rows.generated_images {
        push_media_file(
            &mut entries,
            &mut seen_package_paths,
            "generated_image",
            image.id,
            FILE_PATH_FIELD,
            &image.file_path,
        )?;
    }
    for video in &rows.generated_videos {
        for (field, original_path) in [
            (FILE_PATH_FIELD, video.file_path.as_deref()),
            (THUMBNAIL_PATH_FIELD, video.thumbnail_path.as_deref()),
        ] {
            let Some(original_path) = original_path else {
                continue;
            };
            push_media_file(
                &mut entries,
                &mut seen_package_paths,
                CanvasNodeRefKind::GeneratedVideo.as_str(),
                video.id,
                field,
                original_path,
            )?;
        }
    }
    for portrait in &rows.character_portraits {
        push_media_file(
            &mut entries,
            &mut seen_package_paths,
            "character_portrait",
            portrait.id,
            "image_path",
            &portrait.image_path,
        )?;
    }
    Ok(entries)
}

fn push_media_file(
    entries: &mut Vec<MediaFileEntry>,
    seen_package_paths: &mut HashSet<String>,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    original_path: &str,
) -> Result<(), String> {
    let package_path = family_media_package_path(owner_kind, owner_id, field, original_path)?;
    if !seen_package_paths.insert(package_path.clone()) {
        return Err(format!("项目族导出媒体包路径重复: {package_path}"));
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

pub(crate) fn imported_project_family_media_path(
    project_id: i64,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    package_path: &str,
) -> Result<String, String> {
    let extension = required_path_extension(package_path, "媒体文件")?;
    Ok(format!(
        "projects/{project_id}/media/{owner_kind}/imported_package_{owner_id}_{field}.{extension}"
    ))
}

#[cfg(test)]
mod tests;
