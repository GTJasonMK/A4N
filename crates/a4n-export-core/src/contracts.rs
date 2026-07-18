use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

use crate::archive::decode_base64;
use crate::errors::ExportRenderError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportDocumentKind {
    SettingsAll,
    GenericJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderJsonInput {
    pub kind: ExportDocumentKind,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub pretty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderJsonOutput {
    pub kind: ExportDocumentKind,
    pub content: String,
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportZipEntryBytes {
    pub path: String,
    pub bytes: Vec<u8>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportZipPackageInput {
    pub manifest_path: String,
    pub manifest_bytes: Vec<u8>,
    pub manifest_label: String,
    #[serde(default)]
    pub entries: Vec<ExportZipEntryBytes>,
    pub package_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderZipEntryInput {
    pub path: String,
    pub bytes_base64: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderZipInput {
    pub manifest_path: String,
    pub manifest_bytes_base64: String,
    pub manifest_label: String,
    #[serde(default)]
    pub entries: Vec<ExportRenderZipEntryInput>,
    pub package_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderZipOutput {
    pub bytes_base64: String,
    pub content_type: String,
}

impl ExportRenderZipInput {
    pub fn from_package_input(input: ExportZipPackageInput) -> Self {
        Self {
            manifest_path: input.manifest_path,
            manifest_bytes_base64: BASE64.encode(input.manifest_bytes),
            manifest_label: input.manifest_label,
            entries: input
                .entries
                .into_iter()
                .map(|entry| ExportRenderZipEntryInput {
                    path: entry.path,
                    bytes_base64: BASE64.encode(entry.bytes),
                    label: entry.label,
                })
                .collect(),
            package_label: input.package_label,
        }
    }

    pub(crate) fn into_package_input(self) -> Result<ExportZipPackageInput, ExportRenderError> {
        Ok(ExportZipPackageInput {
            manifest_path: self.manifest_path,
            manifest_bytes: decode_base64(
                self.manifest_label.clone(),
                &self.manifest_bytes_base64,
            )?,
            manifest_label: self.manifest_label,
            entries: self
                .entries
                .into_iter()
                .map(|entry| {
                    let bytes = decode_base64(entry.label.clone(), &entry.bytes_base64)?;
                    Ok(ExportZipEntryBytes {
                        path: entry.path,
                        bytes,
                        label: entry.label,
                    })
                })
                .collect::<Result<Vec<_>, ExportRenderError>>()?,
            package_label: self.package_label,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<ExportRenderJsonInput>(serde_json::json!({
            "kind": "generic_json",
            "payload": {},
            "legacy_mode": true
        }))
        .expect_err("unknown export fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_mode`"));
    }
}

impl ExportRenderZipOutput {
    pub fn into_zip_bytes(self) -> Result<Vec<u8>, ExportRenderError> {
        decode_base64("导出 zip", &self.bytes_base64)
    }
}
