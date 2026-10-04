use std::collections::HashSet;

use super::{family_media_package_path, media_asset_path, required_path_extension};
use a4n_api_contract::project_transfer::{
    MEDIA_ASSET_FILE_PATH_FIELD as FILE_PATH_FIELD,
    MEDIA_ASSET_THUMBNAIL_PATH_FIELD as THUMBNAIL_PATH_FIELD, MediaFileEntry,
};
use a4n_db::project::ProjectFamilyExportRows;
use a4n_domain::CanvasNodeRefKind;

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
