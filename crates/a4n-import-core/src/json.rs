use serde::de::DeserializeOwned;

use crate::contracts::{ImportNormalizeJsonInput, ImportNormalizeJsonOutput};
use crate::errors::ImportTransformError;

pub fn normalize_json(
    input: ImportNormalizeJsonInput,
) -> Result<ImportNormalizeJsonOutput, ImportTransformError> {
    let normalized = serde_json::from_str::<serde_json::Value>(&input.content)?;
    Ok(ImportNormalizeJsonOutput {
        kind: input.kind,
        normalized,
        warnings: Vec::new(),
    })
}

pub fn parse_json_manifest<T>(
    manifest_bytes: &[u8],
    label: impl Into<String>,
) -> Result<T, ImportTransformError>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(manifest_bytes).map_err(|source| {
        ImportTransformError::InvalidManifestJson {
            label: label.into(),
            source,
        }
    })
}

pub fn parse_json_manifest_value<T>(
    manifest: serde_json::Value,
    label: impl Into<String>,
) -> Result<T, ImportTransformError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(manifest).map_err(|source| ImportTransformError::InvalidManifestJson {
        label: label.into(),
        source,
    })
}
