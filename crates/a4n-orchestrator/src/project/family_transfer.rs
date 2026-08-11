use std::{collections::HashSet, path::Path};

use a4n_core::{error::AppError, models::project::novel_project};
use a4n_db::project::ProjectFamilyExportRows;
use async_trait::async_trait;

use super::{
    FamilyImportMediaPayload, MEDIA_FILE_FIELDS, MediaFileEntry, ProjectFamilyPackage,
    family_media_package_path, media_asset_path,
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

#[derive(Debug)]
pub struct ExportedProjectFamilyPackage {
    pub project: novel_project::Model,
    pub package_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct ImportProjectFamilyPackageApplicationInput {
    pub user_id: i64,
    pub package_bytes: Vec<u8>,
}

pub struct ReadProjectFamilyPackage {
    pub package: ProjectFamilyPackage,
    pub media_payload: FamilyImportMediaPayload,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectFamilyTransferError {
    #[error("{0}")]
    Application(#[from] AppError),
    #[error("{0}")]
    Message(String),
}

impl From<String> for ProjectFamilyTransferError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

#[async_trait]
pub trait ProjectFamilyTransferHostPort: Send + Sync {
    async fn render_export_package(
        &self,
        package: &ProjectFamilyPackage,
    ) -> Result<Vec<u8>, String>;

    async fn read_import_package(
        &self,
        package_bytes: Vec<u8>,
    ) -> Result<ReadProjectFamilyPackage, String>;

    fn write_imported_media_file(
        &mut self,
        relative_path: &str,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), String>;

    async fn rollback_imported_media_files(&mut self, label: &str) -> Result<(), String>;
}

pub(crate) fn project_family_export_media_files(
    rows: &ProjectFamilyExportRows,
) -> Result<Vec<MediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for asset in &rows.media_assets {
        for (field, package_field) in MEDIA_FILE_FIELDS {
            let Some(original_path) = media_asset_path(asset, field)? else {
                continue;
            };
            let package_path = family_media_package_path(asset.id, package_field, original_path)?;
            if !seen_package_paths.insert(package_path.clone()) {
                return Err(format!("项目族导出媒体包路径重复: {package_path}"));
            }
            entries.push(MediaFileEntry {
                media_asset_id: asset.id,
                field: field.to_string(),
                original_path: original_path.to_string(),
                package_path,
            });
        }
    }
    Ok(entries)
}

pub(crate) fn imported_project_family_media_path(
    project_id: i64,
    media_kind: &str,
    media_asset_id: i64,
    field: &str,
    package_path: &str,
) -> Result<String, String> {
    let extension = Path::new(package_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {package_path}"))?;
    let suffix = if field == super::MEDIA_FILE_FIELD_THUMBNAIL {
        "_thumbnail"
    } else {
        ""
    };
    Ok(format!(
        "projects/{project_id}/media/{media_kind}/imported_package_{media_asset_id}{suffix}.{extension}"
    ))
}

#[cfg(test)]
mod tests;
