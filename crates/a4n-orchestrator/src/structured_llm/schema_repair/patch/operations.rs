use a4n_core::error::AppError;
use serde_json::Value;

use super::pointer::object_parent_mut;
use crate::structured_llm::schema_repair::contract::{
    SchemaFieldPatchOperationV1, SchemaFieldReplacementV1,
};

pub(super) fn apply_replacement(
    candidate: &mut Value,
    repair: &SchemaFieldReplacementV1,
) -> Result<(), AppError> {
    match repair.op {
        SchemaFieldPatchOperationV1::Replace => {
            let slot = candidate.pointer_mut(&repair.path).ok_or_else(|| {
                AppError::InvalidArgument(format!("replace 路径不存在：{}", repair.path))
            })?;
            *slot = repair.value.clone();
        }
        SchemaFieldPatchOperationV1::Add => {
            let (parent, key) = object_parent_mut(candidate, &repair.path)?;
            if parent.contains_key(&key) {
                return Err(AppError::InvalidArgument(format!(
                    "add 路径已存在：{}",
                    repair.path
                )));
            }
            parent.insert(key, repair.value.clone());
        }
        SchemaFieldPatchOperationV1::Remove => {
            let (parent, key) = object_parent_mut(candidate, &repair.path)?;
            if parent.remove(&key).is_none() {
                return Err(AppError::InvalidArgument(format!(
                    "remove 路径不存在：{}",
                    repair.path
                )));
            }
        }
    }
    Ok(())
}
