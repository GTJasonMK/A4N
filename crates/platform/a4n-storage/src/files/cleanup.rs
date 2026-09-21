use std::{future::Future, io::ErrorKind, path::Path};

use a4n_error::append_secondary_message;

use super::paths::canonicalize_data_file_path;

pub async fn remove_created_file_on_error<T, F>(
    path: &Path,
    label: &str,
    operation: F,
) -> Result<T, String>
where
    F: Future<Output = Result<T, String>>,
{
    match operation.await {
        Ok(value) => Ok(value),
        Err(error) => Err(created_file_error_with_cleanup(path, label, error)),
    }
}

pub fn remove_created_file_on_error_sync<T, F>(
    path: &Path,
    label: &str,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    match operation() {
        Ok(value) => Ok(value),
        Err(error) => Err(created_file_error_with_cleanup(path, label, error)),
    }
}

pub fn remove_existing_data_file(
    data_dir: &Path,
    relative_path: &str,
    label: &str,
) -> Result<(), String> {
    let path = canonicalize_data_file_path(data_dir, relative_path, label)?;
    std::fs::remove_file(&path).map_err(|error| format!("删除{label}失败：{error}"))
}

fn created_file_error_with_cleanup(path: &Path, label: &str, error: String) -> String {
    append_secondary_message(error, remove_created_file(path, label).err())
}

fn remove_created_file(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("同时删除{label}失败：{error}")),
    }
}
