use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredRepairMode {
    None,
    FieldPatch,
    FullObjectRepair,
}

impl StructuredRepairMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::FieldPatch => "field_patch",
            Self::FullObjectRepair => "full_object_repair",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(super) enum SchemaFieldPatchOperationV1 {
    Add,
    Replace,
    Remove,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct SchemaFieldReplacementV1 {
    pub diagnostic_id: String,
    pub op: SchemaFieldPatchOperationV1,
    pub path: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct SchemaFieldPatchV1 {
    pub schema_version: String,
    pub repairs: Vec<SchemaFieldReplacementV1>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(super) struct StructuredJsonDiagnostic {
    pub diagnostic_id: String,
    pub path: String,
    pub operation: Option<SchemaFieldPatchOperationV1>,
    pub keyword: String,
    pub message: String,
    pub expected_schema: Value,
    pub actual_value: Value,
}

#[derive(Debug, Clone)]
pub(super) struct StructuredSchemaReport {
    pub schema_sha256: String,
    pub diagnostics: Vec<StructuredJsonDiagnostic>,
    pub field_patch_allowed: bool,
}
