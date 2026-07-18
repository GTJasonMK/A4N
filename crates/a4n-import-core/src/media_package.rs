use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};

use zip::ZipArchive;

use crate::archive::{
    decode_base64, encode_base64, read_json_manifest_entry, read_zip_entries_bytes,
    validate_zip_manifest_declared_files,
};
use crate::contracts::{
    ImportReadMediaPackageZipEntryOutput, ImportReadMediaPackageZipInput,
    ImportReadMediaPackageZipOutput,
};
use crate::errors::ImportTransformError;

pub fn read_media_package_zip(
    input: ImportReadMediaPackageZipInput,
) -> Result<ImportReadMediaPackageZipOutput, ImportTransformError> {
    let zip_bytes = decode_base64(input.package_label.clone(), &input.zip_bytes_base64)?;
    let (manifest, entries) = read_media_package_zip_bytes(
        &zip_bytes,
        &input.manifest_path,
        input.package_label,
        input.manifest_label,
    )?;
    Ok(ImportReadMediaPackageZipOutput {
        manifest,
        entries: entries
            .into_iter()
            .map(|(path, bytes)| ImportReadMediaPackageZipEntryOutput {
                path,
                bytes_base64: encode_base64(bytes),
            })
            .collect(),
    })
}

pub fn read_media_package_zip_bytes(
    zip_bytes: &[u8],
    manifest_path: &str,
    package_label: impl Into<String>,
    manifest_label: impl Into<String>,
) -> Result<(serde_json::Value, HashMap<String, Vec<u8>>), ImportTransformError> {
    read_media_package_zip_reader(
        Cursor::new(zip_bytes),
        manifest_path,
        package_label,
        manifest_label,
    )
}

pub fn read_media_package_zip_reader<R>(
    reader: R,
    manifest_path: &str,
    package_label: impl Into<String>,
    manifest_label: impl Into<String>,
) -> Result<(serde_json::Value, HashMap<String, Vec<u8>>), ImportTransformError>
where
    R: Read + Seek,
{
    let package_label = package_label.into();
    let manifest_label = manifest_label.into();
    let mut zip = ZipArchive::new(reader).map_err(|source| ImportTransformError::OpenZip {
        label: package_label.clone(),
        source,
    })?;
    let manifest: serde_json::Value = read_json_manifest_entry(
        &mut zip,
        manifest_path,
        package_label.clone(),
        manifest_label,
    )?;
    let declared_files = media_package_declared_files(&manifest, &package_label)?;
    validate_zip_manifest_declared_files(
        &mut zip,
        manifest_path,
        declared_files.iter().map(String::as_str),
        package_label.clone(),
    )?;
    let entries = read_zip_entries_bytes(
        &mut zip,
        declared_files.iter().map(String::as_str),
        package_label,
    )?;
    Ok((manifest, entries))
}

fn media_package_declared_files(
    manifest: &serde_json::Value,
    label: &str,
) -> Result<Vec<String>, ImportTransformError> {
    let media_files = manifest
        .get("media_files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ImportTransformError::InvalidManifestShape {
            label: label.to_string(),
            message: "media_files 必须是数组".to_string(),
        })?;

    let mut paths = Vec::with_capacity(media_files.len());
    for (index, entry) in media_files.iter().enumerate() {
        let path = entry
            .get("package_path")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| ImportTransformError::InvalidManifestShape {
                label: label.to_string(),
                message: format!("media_files[{index}].package_path 必须是非空字符串"),
            })?;
        paths.push(path.to_string());
    }
    Ok(paths)
}
