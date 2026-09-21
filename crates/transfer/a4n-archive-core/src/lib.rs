//! Shared archive invariants for pure import/export transforms.
//!
//! This focused crate is the long-term owner of archive-level rules that must
//! stay identical on import and export paths. It intentionally does not own ZIP
//! encoding/decoding, manifests, media package semantics, database access,
//! permissions, projects, providers, or sidecar transport. Import/export cores
//! compose these invariants into their direction-specific workflows.

use std::collections::HashSet;
use std::io::{Cursor, Write};
use std::path::{Component, Path};
use zip::ZipWriter;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry {
    pub path: String,
    pub bytes: Vec<u8>,
    pub label: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveWriteError {
    #[error("{0}")]
    InvalidEntryPath(#[from] ArchivePathError),
    #[error("写入 zip 条目失败：{path}: {source}")]
    StartEntry {
        path: String,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("导出 zip 条目重复：{path}")]
    DuplicateEntry { path: String },
    #[error("写入{label}失败：{source}")]
    EntryWrite {
        label: String,
        #[source]
        source: std::io::Error,
    },
    #[error("完成{label}失败：{source}")]
    Finish {
        label: String,
        #[source]
        source: zip::result::ZipError,
    },
}

pub fn write_zip(
    entries: impl IntoIterator<Item = ArchiveEntry>,
    package_label: impl Into<String>,
) -> Result<Vec<u8>, ArchiveWriteError> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut paths = HashSet::new();

    for entry in entries {
        validate_zip_entry_path(&entry.path)?;
        if !paths.insert(entry.path.clone()) {
            return Err(ArchiveWriteError::DuplicateEntry { path: entry.path });
        }
        writer.start_file(&entry.path, options).map_err(|source| {
            ArchiveWriteError::StartEntry {
                path: entry.path,
                source,
            }
        })?;
        writer
            .write_all(&entry.bytes)
            .map_err(|source| ArchiveWriteError::EntryWrite {
                label: entry.label,
                source,
            })?;
    }

    writer
        .finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|source| ArchiveWriteError::Finish {
            label: package_label.into(),
            source,
        })
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

    #[test]
    fn write_zip_rejects_duplicate_paths() {
        let error = write_zip(
            [
                ArchiveEntry {
                    path: "manifest.json".into(),
                    bytes: b"{}".to_vec(),
                    label: "manifest".into(),
                },
                ArchiveEntry {
                    path: "manifest.json".into(),
                    bytes: b"{}".to_vec(),
                    label: "duplicate".into(),
                },
            ],
            "package",
        )
        .expect_err("duplicate paths must fail");
        assert!(matches!(
            error,
            ArchiveWriteError::DuplicateEntry { path } if path == "manifest.json"
        ));
    }

    #[test]
    fn write_zip_returns_a_valid_archive() {
        let bytes = write_zip(
            [ArchiveEntry {
                path: "manifest.json".into(),
                bytes: b"{}".to_vec(),
                label: "manifest".into(),
            }],
            "package",
        )
        .expect("archive should render");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip should open");
        let mut manifest = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("manifest.json").unwrap(),
            &mut manifest,
        )
        .expect("manifest should be readable");
        assert_eq!(manifest, "{}");
    }
}
