use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;

use super::super::MAX_SCHEMA_PATCH_BYTES;
use crate::structured_llm::schema_repair::contract::{
    SchemaFieldPatchOperationV1, SchemaFieldPatchV1, StructuredSchemaReport,
};

pub(super) fn validate_schema_field_patch(
    report: &StructuredSchemaReport,
    patch: &SchemaFieldPatchV1,
) -> Result<(), AppError> {
    if patch.schema_version != "structured_json_field_patch_v1" {
        return Err(AppError::InvalidArgument(format!(
            "字段补丁 schema_version 非法：{}",
            patch.schema_version
        )));
    }
    if !report.field_patch_allowed {
        return Err(AppError::InvalidArgument(
            "当前结构错误不能安全使用字段补丁".to_string(),
        ));
    }
    let patch_bytes = serde_json::to_vec(patch)?;
    if patch_bytes.len() > MAX_SCHEMA_PATCH_BYTES {
        return Err(AppError::InvalidArgument(format!(
            "字段补丁过大：{} bytes，最大允许 {} bytes",
            patch_bytes.len(),
            MAX_SCHEMA_PATCH_BYTES
        )));
    }
    if patch.repairs.len() != report.diagnostics.len() {
        return Err(AppError::InvalidArgument(format!(
            "字段补丁数量必须与诊断数量一致：补丁 {}，诊断 {}",
            patch.repairs.len(),
            report.diagnostics.len()
        )));
    }

    let diagnostics = report
        .diagnostics
        .iter()
        .map(|item| (item.diagnostic_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut seen_ids = BTreeSet::new();
    let mut seen_paths = BTreeSet::new();
    for repair in &patch.repairs {
        if !seen_ids.insert(repair.diagnostic_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "字段补丁 diagnostic_id 重复：{}",
                repair.diagnostic_id
            )));
        }
        if !seen_paths.insert(repair.path.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "字段补丁 path 重复：{}",
                repair.path
            )));
        }
        let diagnostic = diagnostics
            .get(repair.diagnostic_id.as_str())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "字段补丁引用未知 diagnostic_id：{}",
                    repair.diagnostic_id
                ))
            })?;
        if diagnostic.path != repair.path || diagnostic.operation != Some(repair.op) {
            return Err(AppError::InvalidArgument(format!(
                "字段补丁越权：{} 只允许 {:?} {}",
                repair.diagnostic_id, diagnostic.operation, diagnostic.path
            )));
        }
        if repair.path.is_empty() || repair.path == "/" {
            return Err(AppError::InvalidArgument(
                "字段补丁禁止修改 JSON 根对象".to_string(),
            ));
        }
        if repair.op == SchemaFieldPatchOperationV1::Remove && !repair.value.is_null() {
            return Err(AppError::InvalidArgument(format!(
                "remove 补丁 {} 的 value 必须是 null",
                repair.diagnostic_id
            )));
        }
    }
    Ok(())
}
