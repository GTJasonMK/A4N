use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::super::{MAX_LOCAL_CONTEXT_BYTES, MAX_SCHEMA_DIAGNOSTICS};
use crate::structured_llm::schema_repair::contract::{
    SchemaFieldPatchOperationV1, StructuredJsonDiagnostic, StructuredSchemaReport,
};

pub(super) struct DiagnosticCollector<'a> {
    diagnostics: Vec<StructuredJsonDiagnostic>,
    seen: BTreeSet<(String, Option<SchemaFieldPatchOperationV1>)>,
    value: &'a Value,
    schema: &'a Value,
    field_patch_allowed: bool,
}

impl<'a> DiagnosticCollector<'a> {
    pub(super) fn new(value: &'a Value, schema: &'a Value) -> Self {
        Self {
            diagnostics: Vec::new(),
            seen: BTreeSet::new(),
            value,
            schema,
            field_patch_allowed: true,
        }
    }

    pub(super) fn disallow_field_patch(&mut self) {
        self.field_patch_allowed = false;
    }

    pub(super) fn push(
        &mut self,
        schema_path: &str,
        path: String,
        operation: Option<SchemaFieldPatchOperationV1>,
        keyword: &str,
        message: String,
    ) {
        if !self.seen.insert((path.clone(), operation)) {
            return;
        }
        let expected_schema = expected_schema_fragment(self.schema, schema_path);
        let actual_value = match operation {
            Some(SchemaFieldPatchOperationV1::Add) => parent_pointer(&path)
                .and_then(|parent| self.value.pointer(parent))
                .cloned()
                .unwrap_or(Value::Null),
            _ => self.value.pointer(&path).cloned().unwrap_or(Value::Null),
        };
        self.diagnostics.push(StructuredJsonDiagnostic {
            diagnostic_id: String::new(),
            path,
            operation,
            keyword: keyword.to_string(),
            message,
            expected_schema,
            actual_value,
        });
    }

    pub(super) fn finish(mut self, schema_sha256: String) -> StructuredSchemaReport {
        if self.diagnostics.is_empty() || self.diagnostics.len() > MAX_SCHEMA_DIAGNOSTICS {
            self.field_patch_allowed = false;
        }
        if self.diagnostics.iter().any(|item| {
            item.operation.is_none()
                || serialized_len(&item.actual_value) > MAX_LOCAL_CONTEXT_BYTES
                || serialized_len(&item.expected_schema) > MAX_LOCAL_CONTEXT_BYTES
        }) {
            self.field_patch_allowed = false;
        }

        for (index, diagnostic) in self.diagnostics.iter_mut().enumerate() {
            diagnostic.diagnostic_id = format!("d{}", index + 1);
        }

        StructuredSchemaReport {
            schema_sha256,
            diagnostics: self.diagnostics,
            field_patch_allowed: self.field_patch_allowed,
        }
    }
}

pub(super) fn required_property_schema_path(schema_path: &str, property: &str) -> String {
    let parent = parent_pointer(schema_path).unwrap_or("");
    format!(
        "{parent}/properties/{}",
        escape_json_pointer_token(property)
    )
}

pub(super) fn join_json_pointer(parent: &str, token: &str) -> String {
    format!("{parent}/{}", escape_json_pointer_token(token))
}

pub(super) fn keyword_from_schema_path(schema_path: &str) -> &str {
    schema_path.rsplit('/').next().unwrap_or("schema")
}

fn expected_schema_fragment(schema: &Value, schema_path: &str) -> Value {
    if let Some(value) = schema.pointer(schema_path)
        && value.is_object()
    {
        return value.clone();
    }
    let parent = parent_pointer(schema_path).unwrap_or("");
    schema.pointer(parent).cloned().unwrap_or_else(|| json!({}))
}

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn parent_pointer(path: &str) -> Option<&str> {
    let index = path.rfind('/')?;
    Some(&path[..index])
}

fn serialized_len(value: &Value) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |bytes| bytes.len())
}
