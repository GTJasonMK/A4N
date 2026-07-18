use a4n_core::error::AppError;
use jsonschema::error::ValidationErrorKind;
use serde_json::Value;

use super::{
    collector::{
        DiagnosticCollector, join_json_pointer, keyword_from_schema_path,
        required_property_schema_path,
    },
    schema::structured_schema_sha256,
};
use crate::structured_llm::schema_repair::contract::{
    SchemaFieldPatchOperationV1, StructuredSchemaReport,
};

pub(in crate::structured_llm::schema_repair) fn diagnose_structured_json_with_schema(
    value: &Value,
    schema: Value,
) -> Result<StructuredSchemaReport, AppError> {
    let schema_sha256 = structured_schema_sha256(&schema)?;
    let validator = jsonschema::validator_for(&schema).map_err(|error| {
        AppError::Internal(anyhow::anyhow!("编译结构化输出 JSON Schema 失败：{error}"))
    })?;
    let mut collector = DiagnosticCollector::new(value, &schema);

    for error in validator.iter_errors(value) {
        let instance_path = error.instance_path().to_string();
        let schema_path = error.schema_path().to_string();
        match error.kind() {
            ValidationErrorKind::AdditionalProperties { unexpected }
            | ValidationErrorKind::UnevaluatedProperties { unexpected } => {
                for property in unexpected {
                    collector.push(
                        &schema_path,
                        join_json_pointer(&instance_path, property),
                        Some(SchemaFieldPatchOperationV1::Remove),
                        keyword_from_schema_path(&schema_path),
                        error.to_string(),
                    );
                }
            }
            ValidationErrorKind::Required { property } => {
                let Some(property) = property.as_str() else {
                    collector.disallow_field_patch();
                    continue;
                };
                collector.push(
                    &required_property_schema_path(&schema_path, property),
                    join_json_pointer(&instance_path, property),
                    Some(SchemaFieldPatchOperationV1::Add),
                    keyword_from_schema_path(&schema_path),
                    error.to_string(),
                );
            }
            ValidationErrorKind::AnyOf { .. }
            | ValidationErrorKind::OneOfMultipleValid { .. }
            | ValidationErrorKind::OneOfNotValid { .. }
            | ValidationErrorKind::FalseSchema
            | ValidationErrorKind::Referencing(_) => {
                collector.disallow_field_patch();
                collector.push(
                    &schema_path,
                    instance_path,
                    None,
                    keyword_from_schema_path(&schema_path),
                    error.to_string(),
                );
            }
            _ => {
                let operation =
                    (!instance_path.is_empty()).then_some(SchemaFieldPatchOperationV1::Replace);
                if operation.is_none() {
                    collector.disallow_field_patch();
                }
                collector.push(
                    &schema_path,
                    instance_path,
                    operation,
                    keyword_from_schema_path(&schema_path),
                    error.to_string(),
                );
            }
        }
    }

    Ok(collector.finish(schema_sha256))
}
