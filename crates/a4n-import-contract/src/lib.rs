//! Wire contracts for the import capability.
//!
//! This crate intentionally contains no ZIP reader, parser, database, or Host
//! state so clients can use the import service without linking its renderer.

use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ImportContractError {
    #[error("{label} base64 解码失败：{source}")]
    Base64Decode {
        label: String,
        #[source]
        source: base64::DecodeError,
    },
    #[error("{label} zip 条目重复：{path}")]
    DuplicateZipEntry { label: String, path: String },
}

fn decode_base64(label: impl Into<String>, value: &str) -> Result<Vec<u8>, ImportContractError> {
    BASE64
        .decode(value)
        .map_err(|source| ImportContractError::Base64Decode {
            label: label.into(),
            source,
        })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportReadMediaPackageZipInput {
    pub zip_bytes_base64: String,
    pub manifest_path: String,
    pub package_label: String,
    pub manifest_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportReadMediaPackageZipEntryOutput {
    pub path: String,
    pub bytes_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportReadMediaPackageZipOutput {
    pub manifest: serde_json::Value,
    pub entries: Vec<ImportReadMediaPackageZipEntryOutput>,
}

impl ImportReadMediaPackageZipInput {
    pub fn from_zip_bytes(
        zip_bytes: Vec<u8>,
        manifest_path: impl Into<String>,
        package_label: impl Into<String>,
        manifest_label: impl Into<String>,
    ) -> Self {
        Self {
            zip_bytes_base64: BASE64.encode(zip_bytes),
            manifest_path: manifest_path.into(),
            package_label: package_label.into(),
            manifest_label: manifest_label.into(),
        }
    }
}

impl ImportReadMediaPackageZipOutput {
    pub fn into_manifest_and_entries_bytes(
        self,
    ) -> Result<(serde_json::Value, HashMap<String, Vec<u8>>), ImportContractError> {
        let mut entries = HashMap::new();
        for entry in self.entries {
            let bytes = decode_base64(entry.path.clone(), &entry.bytes_base64)?;
            if entries.insert(entry.path.clone(), bytes).is_some() {
                return Err(ImportContractError::DuplicateZipEntry {
                    label: "导入媒体项目包".to_string(),
                    path: entry.path,
                });
            }
        }
        Ok((self.manifest, entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<ImportReadMediaPackageZipInput>(serde_json::json!({
            "zip_bytes_base64": "UEsDBAo=",
            "manifest_path": "manifest.json",
            "package_label": "测试包",
            "manifest_label": "测试清单",
            "legacy_mode": true
        }))
        .expect_err("unknown import fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_mode`"));
        assert!(
            serde_json::from_value::<ImportReadMediaPackageZipOutput>(serde_json::json!({
                "manifest": {}
            }))
            .is_err()
        );
    }
}
