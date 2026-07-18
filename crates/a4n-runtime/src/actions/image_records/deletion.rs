use crate::{
    AppState,
    actions::{access, app_error_message},
    file_utils::deletable_data_file_path,
};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::rules::append_secondary_error;

pub async fn delete_image(state: &AppState, user_id: i64, image_id: i64) -> Result<(), String> {
    let existing = state
        .orchestrator
        .generated_image_by_id(image_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| "图片不存在".to_string())?;
    access::verify_project_editor_string(state, user_id, existing.project_id).await?;
    let image_path =
        deletable_data_file_path(&state.config.data_dir, &existing.file_path, "图片文件")?;
    let staged_file = match image_path.as_ref() {
        Some(path) => Some(stage_image_delete_file(path)?),
        None => None,
    };
    let delete_result = state
        .orchestrator
        .delete_generated_image_record(image_id)
        .await
        .map_err(app_error_message);
    match delete_result {
        Ok(()) => {
            if let Some(staged_file) = staged_file {
                staged_file.remove()?;
            }
            Ok(())
        }
        Err(err) => {
            let restore_error = staged_file.and_then(|staged_file| staged_file.restore().err());
            Err(append_secondary_error(
                format!("图片记录删除失败：{err}"),
                restore_error,
            ))
        }
    }
}

pub(super) fn remove_image_delete_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("图片文件删除失败：{}：{err}", path.display())),
    }
}

pub(super) struct StagedImageDeleteFile {
    original_path: PathBuf,
    pub(super) staged_path: PathBuf,
    pub(super) staging_dir: PathBuf,
}

impl StagedImageDeleteFile {
    pub(super) fn remove(self) -> Result<(), String> {
        let remove_file = remove_image_delete_file(&self.staged_path);
        let remove_dir = remove_image_delete_staging_dir(&self.staging_dir);
        match (remove_file, remove_dir) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(file_err), Ok(())) => Err(file_err),
            (Ok(()), Err(dir_err)) => Err(dir_err),
            (Err(file_err), Err(dir_err)) => Err(format!("{file_err}；{dir_err}")),
        }
    }

    pub(super) fn restore(self) -> Result<(), String> {
        let restore = std::fs::rename(&self.staged_path, &self.original_path).map_err(|err| {
            format!(
                "图片记录删除失败后恢复文件失败：{} -> {}：{err}",
                self.staged_path.display(),
                self.original_path.display()
            )
        });
        let remove_dir = remove_image_delete_staging_dir(&self.staging_dir);
        match (restore, remove_dir) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(restore_err), Ok(())) => Err(restore_err),
            (Ok(()), Err(dir_err)) => Err(dir_err),
            (Err(restore_err), Err(dir_err)) => Err(format!("{restore_err}；{dir_err}")),
        }
    }
}

pub(super) fn stage_image_delete_file(path: &Path) -> Result<StagedImageDeleteFile, String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("图片文件缺少父目录：{}", path.display()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("图片文件缺少文件名：{}", path.display()))?;
    let staging_dir = create_image_delete_staging_dir(parent)?;
    let staged_path = staging_dir.join(file_name);
    match std::fs::rename(path, &staged_path) {
        Ok(()) => Ok(StagedImageDeleteFile {
            original_path: path.to_path_buf(),
            staged_path,
            staging_dir,
        }),
        Err(err) => {
            let primary = format!(
                "图片文件移入删除暂存区失败：{} -> {}：{err}",
                path.display(),
                staged_path.display()
            );
            match std::fs::remove_dir(&staging_dir) {
                Ok(()) => Err(primary),
                Err(cleanup_error) => Err(format!(
                    "{primary}；删除空的图片删除暂存区失败：{}：{cleanup_error}",
                    staging_dir.display()
                )),
            }
        }
    }
}

fn create_image_delete_staging_dir(parent: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..100 {
        let candidate = parent.join(format!(
            ".a4n-delete-image-{}-{stamp}-{attempt}",
            std::process::id()
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "创建图片删除暂存目录失败：{}：{err}",
                    candidate.display()
                ));
            }
        }
    }
    Err(format!(
        "创建图片删除暂存目录失败：{} 下可用名称已耗尽",
        parent.display()
    ))
}

fn remove_image_delete_staging_dir(path: &Path) -> Result<(), String> {
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "图片删除暂存目录删除失败：{}：{err}",
            path.display()
        )),
    }
}
