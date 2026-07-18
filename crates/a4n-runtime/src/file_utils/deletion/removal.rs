use super::validation::{deletable_data_dir_path, deletable_data_file_path};
use std::{io::ErrorKind, path::Path};

pub fn remove_data_file_in_data_dir(
    data_dir: &Path,
    path: &str,
    label: &str,
) -> Result<(), String> {
    let Some(canonical_path) = deletable_data_file_path(data_dir, path, label)? else {
        return Ok(());
    };
    match std::fs::remove_file(&canonical_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "{label}删除失败：{}：{error}",
            canonical_path.display()
        )),
    }
}

pub fn remove_data_dir_in_data_dir(
    data_dir: &Path,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    let Some(canonical_path) = deletable_data_dir_path(data_dir, path, label)? else {
        return Ok(());
    };
    match std::fs::remove_dir_all(&canonical_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "{label}删除失败：{}：{error}",
            canonical_path.display()
        )),
    }
}
