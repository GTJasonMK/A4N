use crate::file_utils;
use a4n_orchestrator::project::{
    DerivedProjectMediaFileCopier, FreeformProjectMediaFileCopier, ProjectMediaCopyError,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn copy_media_file(
    data_dir: &Path,
    new_project_id: i64,
    source_media_asset_id: i64,
    field: &str,
    relative_path: Option<&str>,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<Option<String>, ProjectMediaCopyError> {
    let Some(relative_path) = relative_path else {
        return Ok(None);
    };
    let source = file_utils::canonicalize_data_file_path(data_dir, relative_path, "媒体文件")
        .map_err(ProjectMediaCopyError::InvalidSourcePath)?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ProjectMediaCopyError::MissingExtension {
            path: relative_path.to_string(),
        })?;
    let target_relative = format!(
        "projects/{new_project_id}/media/duplicated/{source_media_asset_id}_{field}.{extension}"
    );
    let target = data_dir.join(&target_relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|source| ProjectMediaCopyError::CreateDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::copy(&source, &target).map_err(|error| ProjectMediaCopyError::CopyFile {
        source_path: source.clone(),
        target_path: target.clone(),
        source: error,
    })?;
    copied_media_files.push(target);
    Ok(Some(target_relative))
}

pub(super) struct RuntimeProjectMediaFileCopier<'a> {
    pub(super) data_dir: &'a Path,
    pub(super) copied_media_files: &'a mut Vec<PathBuf>,
}

impl RuntimeProjectMediaFileCopier<'_> {
    fn copy_project_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        copy_media_file(
            self.data_dir,
            new_project_id,
            source_media_asset_id,
            field,
            relative_path,
            self.copied_media_files,
        )
    }
}

impl DerivedProjectMediaFileCopier for RuntimeProjectMediaFileCopier<'_> {
    fn copy_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        self.copy_project_media_file(new_project_id, source_media_asset_id, field, relative_path)
    }
}

impl FreeformProjectMediaFileCopier for RuntimeProjectMediaFileCopier<'_> {
    fn copy_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        self.copy_project_media_file(new_project_id, source_media_asset_id, field, relative_path)
    }
}
