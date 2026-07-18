use a4n_core::error::AppError;
use serde_json::Value;

use super::{operations::apply_replacement, validation::validate_schema_field_patch};
use crate::structured_llm::schema_repair::contract::{SchemaFieldPatchV1, StructuredSchemaReport};

pub(in crate::structured_llm::schema_repair) fn apply_schema_field_patch(
    base: &Value,
    report: &StructuredSchemaReport,
    patch: &SchemaFieldPatchV1,
) -> Result<Value, AppError> {
    validate_schema_field_patch(report, patch)?;

    let mut candidate = base.clone();
    for repair in &patch.repairs {
        apply_replacement(&mut candidate, repair)?;
    }
    Ok(candidate)
}
