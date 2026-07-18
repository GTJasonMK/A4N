use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::archive::{decode_base64, encode_base64};
use crate::errors::ImportTransformError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportDocumentKind {
    SettingsAll,
    GenericJson,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportNormalizeJsonInput {
    pub kind: ImportDocumentKind,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportNormalizeJsonOutput {
    pub kind: ImportDocumentKind,
    pub normalized: serde_json::Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
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
    #[serde(default)]
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
            zip_bytes_base64: encode_base64(zip_bytes),
            manifest_path: manifest_path.into(),
            package_label: package_label.into(),
            manifest_label: manifest_label.into(),
        }
    }
}

impl ImportReadMediaPackageZipOutput {
    pub fn into_manifest_and_entries_bytes(
        self,
    ) -> Result<(serde_json::Value, HashMap<String, Vec<u8>>), ImportTransformError> {
        let mut entries = HashMap::new();
        for entry in self.entries {
            let bytes = decode_base64(entry.path.clone(), &entry.bytes_base64)?;
            if entries.insert(entry.path.clone(), bytes).is_some() {
                return Err(ImportTransformError::DuplicateZipEntry {
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
        let error = serde_json::from_value::<ImportNormalizeJsonInput>(serde_json::json!({
            "kind": "generic_json",
            "content": "{}",
            "legacy_mode": true
        }))
        .expect_err("unknown import fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_mode`"));
    }
}
