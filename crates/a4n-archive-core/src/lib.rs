//! Shared archive invariants for pure import/export transforms.
//!
//! This focused crate is the long-term owner of archive-level rules that must
//! stay identical on import and export paths. It intentionally does not own ZIP
//! encoding/decoding, manifests, media package semantics, database access,
//! permissions, projects, providers, or sidecar transport. Import/export cores
//! compose these invariants into their direction-specific workflows.

use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArchivePathError {
    #[error("zip 条目路径不能为空")]
    Empty,
    #[error("zip 条目路径不能包含反斜杠: {path}")]
    Backslash { path: String },
    #[error("zip 条目路径不能是绝对路径: {path}")]
    Absolute { path: String },
    #[error("zip 条目路径不能越界: {path}")]
    EscapesRoot { path: String },
}

pub fn validate_zip_entry_path(path: &str) -> Result<(), ArchivePathError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(ArchivePathError::Empty);
    }
    if path.contains('\\') {
        return Err(ArchivePathError::Backslash {
            path: path.to_string(),
        });
    }

    let raw = Path::new(path);
    if raw.is_absolute() {
        return Err(ArchivePathError::Absolute {
            path: path.to_string(),
        });
    }

    let mut has_component = false;
    for component in raw.components() {
        match component {
            Component::Normal(_) => {
                has_component = true;
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(ArchivePathError::EscapesRoot {
                    path: path.to_string(),
                });
            }
        }
    }
    if !has_component {
        return Err(ArchivePathError::Empty);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_zip_entry_path_accepts_nested_relative_path() {
        validate_zip_entry_path("media/media_asset/10/file.mp4").unwrap();
    }

    #[test]
    fn validate_zip_entry_path_accepts_directory_entry_path() {
        validate_zip_entry_path("media/").unwrap();
    }

    #[test]
    fn validate_zip_entry_path_rejects_traversal() {
        let err = validate_zip_entry_path("media/10/../file.mp4").unwrap_err();
        assert!(err.to_string().contains("不能越界"));
    }

    #[test]
    fn validate_zip_entry_path_rejects_current_dir_segment() {
        let err = validate_zip_entry_path("./media/file.mp4").unwrap_err();
        assert!(err.to_string().contains("不能越界"));
    }

    #[test]
    fn validate_zip_entry_path_rejects_backslash() {
        let err = validate_zip_entry_path("media\\file.mp4").unwrap_err();
        assert!(err.to_string().contains("反斜杠"));
    }

    #[test]
    fn validate_zip_entry_path_rejects_absolute_path() {
        let err = validate_zip_entry_path("/media/file.mp4").unwrap_err();
        assert!(err.to_string().contains("绝对路径"));
    }

    #[test]
    fn validate_zip_entry_path_rejects_blank_path() {
        let err = validate_zip_entry_path(" \n\t ").unwrap_err();
        assert!(err.to_string().contains("不能为空"));
    }
}
