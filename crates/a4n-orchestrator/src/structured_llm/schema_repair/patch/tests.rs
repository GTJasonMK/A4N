use serde_json::{Value, json};

use super::apply_schema_field_patch;
use crate::structured_llm::schema_repair::contract::{
    SchemaFieldPatchOperationV1, SchemaFieldPatchV1, SchemaFieldReplacementV1,
    StructuredJsonDiagnostic, StructuredSchemaReport,
};

fn report() -> StructuredSchemaReport {
    StructuredSchemaReport {
        schema_sha256: "schema".into(),
        field_patch_allowed: true,
        diagnostics: vec![
            StructuredJsonDiagnostic {
                diagnostic_id: "d1".into(),
                path: "/names".into(),
                operation: Some(SchemaFieldPatchOperationV1::Replace),
                keyword: "type".into(),
                message: "expected array".into(),
                expected_schema: json!({"type": "array"}),
                actual_value: json!("Alice"),
            },
            StructuredJsonDiagnostic {
                diagnostic_id: "d2".into(),
                path: "/nested".into(),
                operation: Some(SchemaFieldPatchOperationV1::Add),
                keyword: "required".into(),
                message: "required".into(),
                expected_schema: json!({"type": "object"}),
                actual_value: json!({}),
            },
            StructuredJsonDiagnostic {
                diagnostic_id: "d3".into(),
                path: "/extra".into(),
                operation: Some(SchemaFieldPatchOperationV1::Remove),
                keyword: "additionalProperties".into(),
                message: "unexpected".into(),
                expected_schema: json!({}),
                actual_value: json!(true),
            },
        ],
    }
}

#[test]
fn applies_only_authorized_field_operations() {
    let candidate = apply_schema_field_patch(
        &json!({"names": "Alice", "extra": true}),
        &report(),
        &SchemaFieldPatchV1 {
            schema_version: "structured_json_field_patch_v1".into(),
            repairs: vec![
                SchemaFieldReplacementV1 {
                    diagnostic_id: "d1".into(),
                    op: SchemaFieldPatchOperationV1::Replace,
                    path: "/names".into(),
                    value: json!(["Alice"]),
                },
                SchemaFieldReplacementV1 {
                    diagnostic_id: "d2".into(),
                    op: SchemaFieldPatchOperationV1::Add,
                    path: "/nested".into(),
                    value: json!({"title": "x"}),
                },
                SchemaFieldReplacementV1 {
                    diagnostic_id: "d3".into(),
                    op: SchemaFieldPatchOperationV1::Remove,
                    path: "/extra".into(),
                    value: Value::Null,
                },
            ],
        },
    )
    .expect("field patch should apply");

    assert_eq!(
        candidate,
        json!({"names": ["Alice"], "nested": {"title": "x"}})
    );
}

#[test]
fn rejects_path_escalation() {
    let error = apply_schema_field_patch(
        &json!({"names": "Alice", "extra": true}),
        &report(),
        &SchemaFieldPatchV1 {
            schema_version: "structured_json_field_patch_v1".into(),
            repairs: vec![
                SchemaFieldReplacementV1 {
                    diagnostic_id: "d1".into(),
                    op: SchemaFieldPatchOperationV1::Replace,
                    path: "/".into(),
                    value: json!({}),
                },
                SchemaFieldReplacementV1 {
                    diagnostic_id: "d2".into(),
                    op: SchemaFieldPatchOperationV1::Add,
                    path: "/nested".into(),
                    value: json!({}),
                },
                SchemaFieldReplacementV1 {
                    diagnostic_id: "d3".into(),
                    op: SchemaFieldPatchOperationV1::Remove,
                    path: "/extra".into(),
                    value: Value::Null,
                },
            ],
        },
    )
    .expect_err("root replacement should be rejected");

    assert!(error.to_string().contains("越权"));
}
