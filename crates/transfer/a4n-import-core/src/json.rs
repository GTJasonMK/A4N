use serde::de::DeserializeOwned;

use crate::errors::ImportTransformError;

pub(super) fn parse_json_manifest<T>(
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
