use std::{
    future::Future,
    io::ErrorKind,
    path::{Path, PathBuf},
};

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

fn created_file_error_with_cleanup(path: &Path, label: &str, error: String) -> String {
    match remove_created_file(path, label) {
        Ok(()) => error,
        Err(cleanup_error) => format!("{error}；{cleanup_error}"),
    }
}

fn remove_created_file(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("同时删除{label}失败：{error}")),
    }
}

pub(crate) fn error_with_rollback_file_cleanup(
    primary: String,
    paths: &[PathBuf],
    label: &str,
) -> String {
    match rollback_file_cleanup(paths, label) {
        Ok(()) => primary,
        Err(cleanup_error) => format!("{primary}；{cleanup_error}"),
    }
}

pub(crate) fn rollback_file_cleanup(paths: &[PathBuf], label: &str) -> Result<(), String> {
    let mut cleanup_errors = Vec::new();
    for path in paths.iter().rev() {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                cleanup_errors.push(format!("删除{label}失败：{}：{error}", path.display()))
            }
        }
    }
    if cleanup_errors.is_empty() {
        Ok(())
    } else {
        Err(cleanup_errors.join("；"))
    }
}
