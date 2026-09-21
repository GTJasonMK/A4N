use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::de::DeserializeOwned;
use zip::ZipArchive;

use crate::errors::ImportTransformError;
use crate::json::parse_json_manifest;

pub(super) fn read_json_manifest_entry<R, T>(
    zip: &mut ZipArchive<R>,
    manifest_path: &str,
    package_label: impl Into<String>,
    manifest_label: impl Into<String>,
) -> Result<T, ImportTransformError>
where
    R: Read + Seek,
    T: DeserializeOwned,
{
    let manifest_bytes = read_zip_entry_bytes(zip, manifest_path, package_label)?;
    parse_json_manifest(&manifest_bytes, manifest_label)
}

pub(super) fn read_zip_entry_bytes<R>(
    zip: &mut ZipArchive<R>,
    path: &str,
    label: impl Into<String>,
) -> Result<Vec<u8>, ImportTransformError>
where
    R: Read + Seek,
{
    a4n_archive_core::validate_zip_entry_path(path)?;
    let label = label.into();
    let mut file = zip
        .by_name(path)
        .map_err(|source| ImportTransformError::MissingZipEntry {
            label: label.clone(),
            path: path.to_string(),
            source,
        })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|source| ImportTransformError::ReadZipEntry {
            label,
            path: path.to_string(),
            source,
        })?;
    Ok(bytes)
}

pub(super) fn read_zip_entries_bytes<R, I, S>(
    zip: &mut ZipArchive<R>,
    paths: I,
    label: impl Into<String>,
) -> Result<HashMap<String, Vec<u8>>, ImportTransformError>
where
    R: Read + Seek,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let label = label.into();
    let mut entries = HashMap::new();
    for path in paths {
        let path = path.as_ref();
        if entries.contains_key(path) {
            return Err(ImportTransformError::DuplicateDeclaredZipEntry {
                label,
                path: path.to_string(),
            });
        }
        let bytes = read_zip_entry_bytes(zip, path, label.clone())?;
        entries.insert(path.to_string(), bytes);
    }
    Ok(entries)
}

pub(super) fn validate_zip_manifest_declared_files<R, I, S>(
    zip: &mut ZipArchive<R>,
    manifest_path: &str,
    declared_files: I,
    label: impl Into<String>,
) -> Result<(), ImportTransformError>
where
    R: Read + Seek,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut paths = vec![manifest_path.to_string()];
    for path in declared_files {
        paths.push(path.as_ref().to_string());
    }
    validate_zip_declared_files(zip, paths, label)
}

pub(super) fn validate_zip_declared_files<R, I, S>(
    zip: &mut ZipArchive<R>,
    declared_files: I,
    label: impl Into<String>,
) -> Result<(), ImportTransformError>
where
    R: Read + Seek,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let label = label.into();
    let mut declared = HashSet::new();
    for path in declared_files {
        let path = path.as_ref();
        a4n_archive_core::validate_zip_entry_path(path)?;
        if !declared.insert(path.to_string()) {
            return Err(ImportTransformError::DuplicateDeclaredZipEntry {
                label,
                path: path.to_string(),
            });
        }
    }

    let mut seen = HashSet::new();
    for index in 0..zip.len() {
        let file =
            zip.by_index(index)
                .map_err(|source| ImportTransformError::ReadZipEntryByIndex {
                    label: label.clone(),
                    index,
                    source,
                })?;
        let name = file.name().to_string();
        a4n_archive_core::validate_zip_entry_path(&name)?;
        if file.is_dir() {
            continue;
        }
        if !seen.insert(name.clone()) {
            return Err(ImportTransformError::DuplicateZipEntry { label, path: name });
        }
        if !declared.contains(&name) {
            return Err(ImportTransformError::UndeclaredZipEntry { label, path: name });
        }
    }

    Ok(())
}

pub(super) fn encode_base64(value: impl AsRef<[u8]>) -> String {
    BASE64.encode(value)
}

pub(super) fn decode_base64(
    label: impl Into<String>,
    value: &str,
) -> Result<Vec<u8>, ImportTransformError> {
    BASE64
        .decode(value)
        .map_err(|source| ImportTransformError::Base64Decode {
            label: label.into(),
            source,
        })
}
