//! Wire and in-memory data contracts for the export capability.
//!
//! This crate intentionally contains no renderer or archive implementation so
//! Host clients do not rebuild when a Worker renderer changes.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

pub const EXPORT_SERVICE_NAME: &str = "export";
pub const EXPORT_RENDER_CAPABILITY_VERSION: &str = "export-render-v3";
pub const EXPORT_SERVICE_ADDR_ENV: &str = "A4N_EXPORT_SERVICE_ADDR";
pub const EXPORT_SERVICE_DEFAULT_ADDR: &str = "127.0.0.1:47648";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportChapter {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportDocument {
    pub title: String,
    pub chapters: Vec<ExportChapter>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportDocumentFormat {
    Txt,
    Markdown,
    Epub,
    Pdf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderDocumentInput {
    pub document: ExportDocument,
    pub format: ExportDocumentFormat,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderDocumentOutput {
    pub bytes_base64: String,
    pub extension: String,
    pub content_type: String,
}

impl ExportRenderDocumentOutput {
    pub fn into_bytes(self) -> Result<Vec<u8>, ExportContractError> {
        decode_base64("导出文档", &self.bytes_base64)
    }
}

#[async_trait::async_trait]
pub trait ExportDocumentRenderer: Send + Sync {
    async fn render_document(
        &self,
        input: ExportRenderDocumentInput,
        project_id: Option<i64>,
        root_project_id: Option<i64>,
        progress: Option<&dyn ExportRenderObserver>,
    ) -> Result<ExportRenderDocumentOutput, ExportRenderFailure>;
}

/// In-process progress only; transport adapters retain Host-owned cancellation.
pub trait ExportRenderObserver: Send + Sync {
    fn progress(&self, current: u32, total: u32, message: &str);
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum ExportRenderFailure {
    #[error("导出已取消")]
    Cancelled,
    #[error("{0}")]
    Failed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ExportContractError {
    #[error("{label} base64 解码失败：{source}")]
    Base64Decode {
        label: String,
        #[source]
        source: base64::DecodeError,
    },
}

fn decode_base64(label: impl Into<String>, value: &str) -> Result<Vec<u8>, ExportContractError> {
    BASE64
        .decode(value)
        .map_err(|source| ExportContractError::Base64Decode {
            label: label.into(),
            source,
        })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExportRenderJsonInput {
    pub payload: serde_json::Value,
    pub pretty: bool,
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

    pub fn into_package_input(self) -> Result<ExportZipPackageInput, ExportContractError> {
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
                .collect::<Result<Vec<_>, ExportContractError>>()?,
            package_label: self.package_label,
        })
    }
}

impl ExportRenderZipOutput {
    pub fn into_zip_bytes(self) -> Result<Vec<u8>, ExportContractError> {
        decode_base64("导出 zip", &self.bytes_base64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<ExportRenderJsonInput>(serde_json::json!({
            "payload": {},
            "pretty": false,
            "legacy_mode": true
        }))
        .expect_err("unknown export fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_mode`"));
        assert!(
            serde_json::from_value::<ExportRenderJsonInput>(serde_json::json!({
                "payload": {}
            }))
            .is_err()
        );
    }

    #[test]
    fn zip_contract_reports_invalid_base64_with_the_field_label() {
        let error = ExportRenderZipInput {
            manifest_path: "manifest.json".to_string(),
            manifest_bytes_base64: "e30=".to_string(),
            manifest_label: "manifest".to_string(),
            entries: vec![ExportRenderZipEntryInput {
                path: "media/file.txt".to_string(),
                bytes_base64: "not base64".to_string(),
                label: "media file".to_string(),
            }],
            package_label: "package".to_string(),
        };

        let error = error
            .into_package_input()
            .expect_err("invalid entry base64 must fail");
        assert!(error.to_string().contains("media file base64 解码失败"));
    }

    #[test]
    fn document_contract_reports_invalid_base64() {
        let error = ExportRenderDocumentOutput {
            bytes_base64: "not base64".to_string(),
            extension: "txt".to_string(),
            content_type: "text/plain;charset=utf-8".to_string(),
        }
        .into_bytes()
        .expect_err("invalid document base64 must fail");

        assert!(error.to_string().contains("导出文档 base64 解码失败"));
    }
}
