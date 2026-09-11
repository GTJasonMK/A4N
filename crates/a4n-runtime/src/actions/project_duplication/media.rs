use crate::{AppState, actions::access, cleanup_outbox::enqueue_file_artifact_cleanup_by_project};
use a4n_domain::project::{ProjectMediaCopyError, ProjectMediaFileCopier};
use a4n_error::AppError;
use a4n_orchestrator::project::ProjectDuplicationHostPort;
use a4n_storage::{canonicalize_data_file_path, data_dir_relative_file_path};
use async_trait::async_trait;
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
    let source = canonicalize_data_file_path(data_dir, relative_path, "媒体文件")
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
    copied_media_files.push(target.clone());
    fs::copy(&source, &target).map_err(|error| ProjectMediaCopyError::CopyFile {
        source_path: source.clone(),
        target_path: target,
        source: error,
    })?;
    Ok(Some(target_relative))
}

pub(super) struct RuntimeProjectDuplicationHostPort<'a> {
    state: &'a AppState,
    data_dir: PathBuf,
    copied_media_files: Vec<PathBuf>,
    cleanup_token: String,
}

impl<'a> RuntimeProjectDuplicationHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self {
            state,
            data_dir: state.data_dir().to_path_buf(),
            copied_media_files: Vec::new(),
            cleanup_token: uuid::Uuid::new_v4().to_string(),
        }
    }

    fn copy_project_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        copy_media_file(
            &self.data_dir,
            new_project_id,
            source_media_asset_id,
            field,
            relative_path,
            &mut self.copied_media_files,
        )
    }
}

impl ProjectMediaFileCopier for RuntimeProjectDuplicationHostPort<'_> {
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

#[async_trait]
impl ProjectDuplicationHostPort for RuntimeProjectDuplicationHostPort<'_> {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        access::verify_project_editor(self.state, user_id, project_id).await
    }

    async fn rollback_copied_media_files(&mut self, label: &str) -> Result<(), String> {
        let paths = self
            .copied_media_files
            .iter()
            .map(|path| data_dir_relative_file_path(&self.data_dir, path, label))
            .collect::<Result<Vec<_>, _>>()?;
        enqueue_file_artifact_cleanup_by_project(self.state, &self.cleanup_token, &paths).await
    }
}

#[cfg(test)]
pub(super) struct RuntimeProjectMediaFileCopier<'a> {
    pub(super) data_dir: &'a Path,
    pub(super) copied_media_files: &'a mut Vec<PathBuf>,
}

#[cfg(test)]
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

#[cfg(test)]
impl ProjectMediaFileCopier for RuntimeProjectMediaFileCopier<'_> {
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
