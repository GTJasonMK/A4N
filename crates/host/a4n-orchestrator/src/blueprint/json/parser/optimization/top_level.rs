use a4n_domain::project::{BLUEPRINT_REFINEMENT_FIELDS, BLUEPRINT_REFINEMENT_PROMPT};
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::fields::reject_unknown_keys;

pub(super) fn parse_optimization_top_level(content: &str) -> Result<Value, AppError> {
    let value = a4n_domain::json_contract::parse_strict_json_value(content.trim())
        .map_err(|e| AppError::InvalidArgument(format!("模型返回无法解析为蓝图优化 JSON: {e}")))?;
    let Some(object) = value.as_object() else {
        return Err(AppError::InvalidArgument(
            "蓝图优化结果必须是 JSON 对象".to_string(),
        ));
    };
    reject_unknown_keys(
        object,
        BLUEPRINT_REFINEMENT_FIELDS,
        BLUEPRINT_REFINEMENT_PROMPT,
    )?;
    if !BLUEPRINT_REFINEMENT_FIELDS
        .iter()
        .any(|key| object.contains_key(*key))
    {
        return Err(AppError::InvalidArgument(format!(
            "蓝图优化结果缺少可应用字段：{}",
            BLUEPRINT_REFINEMENT_FIELDS.join("、")
        )));
    }
    Ok(value)
}
