use std::{collections::HashSet, path::Path};

use a4n_core::models::media::media_asset;

use crate::{file_utils::canonicalize_data_file_path, state::AppState};

use super::super::{
    MANIFEST_PATH, MEDIA_FILE_FIELDS, MediaFileEntry, ProjectFamilyPackage, media_asset_path,
};

pub(super) fn collect_media_files(
    state: &AppState,
    media_assets: &[media_asset::Model],
) -> Result<Vec<MediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for asset in media_assets {
        for (field, package_field) in MEDIA_FILE_FIELDS {
            let Some(path) = media_asset_path(asset, field)? else {
                continue;
            };
            let package_path = media_package_path(asset.id, package_field, path)?;
            validate_media_file(state, path, &package_path, &mut seen_package_paths)?;
            entries.push(MediaFileEntry {
                media_asset_id: asset.id,
                field: field.to_string(),
                original_path: path.to_string(),
                package_path,
            });
        }
    }
    Ok(entries)
}

fn validate_media_file(
    state: &AppState,
    original_path: &str,
    package_path: &str,
    seen_package_paths: &mut HashSet<String>,
) -> Result<(), String> {
    canonicalize_data_file_path(&state.config.data_dir, original_path, "媒体文件")?;
    if !seen_package_paths.insert(package_path.to_string()) {
        return Err(format!("项目族导出媒体包路径重复: {package_path}"));
    }
    Ok(())
}

fn media_package_path(
    media_asset_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = Path::new(original_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {original_path}"))?;
    Ok(format!("media/{media_asset_id}/{field}.{extension}",))
}

pub(super) async fn package_zip_bytes(
    state: &AppState,
    package: &ProjectFamilyPackage,
) -> Result<Vec<u8>, String> {
    let manifest = crate::sidecar_clients::render_export_json_manifest_bytes(
        state.runtime_profile,
        state.capabilities(),
        package,
        "项目族导出包",
        Some(package.root_project_id),
        Some(package.root_project_id),
    )
    .await?;
    let entries = package_media_zip_entries(state, package)?;
    crate::sidecar_clients::render_export_zip_package(
        state.runtime_profile,
        state.capabilities(),
        a4n_export_core::ExportZipPackageInput {
            manifest_path: MANIFEST_PATH.to_string(),
            manifest_bytes: manifest,
            manifest_label: "项目族清单".to_string(),
            entries,
            package_label: "项目族导出包".to_string(),
        },
        Some(package.root_project_id),
        Some(package.root_project_id),
    )
    .await
}

fn package_media_zip_entries(
    state: &AppState,
    package: &ProjectFamilyPackage,
) -> Result<Vec<a4n_export_core::ExportZipEntryBytes>, String> {
    let mut entries = Vec::with_capacity(package.media_files.len());
    for media in &package.media_files {
        let source =
            canonicalize_data_file_path(&state.config.data_dir, &media.original_path, "媒体文件")?;
        let bytes = std::fs::read(&source)
            .map_err(|err| format!("读取媒体文件失败: {}: {err}", source.display()))?;
        entries.push(a4n_export_core::ExportZipEntryBytes {
            path: media.package_path.clone(),
            bytes,
            label: format!("媒体文件 {}", media.package_path),
        });
    }
    Ok(entries)
}
