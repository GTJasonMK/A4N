use std::collections::HashSet;

use a4n_core::error::AppError;
use serde_json::Value;

use super::super::validate_structured_value;

pub(super) fn validate_reference_binding(
    value: &Value,
    reference_image_ids: &[i64],
) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("视频提示词 reference_binding 必须是对象".to_string())
    })?;
    if object.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 reference_binding 不能为空".to_string(),
        ));
    }
    validate_structured_value(
        object.get("coverage_check").unwrap_or(&Value::Null),
        "视频提示词 reference_binding.coverage_check",
    )?;

    let bindings = match object.get("bindings") {
        Some(value) => Some(value.as_array().ok_or_else(|| {
            AppError::InvalidArgument(
                "视频提示词 reference_binding.bindings 必须是数组".to_string(),
            )
        })?),
        None => None,
    };
    if reference_image_ids.is_empty() {
        if bindings.is_some_and(|items| !items.is_empty()) {
            return Err(AppError::InvalidArgument(
                "视频提示词 reference_binding.bindings 不能绑定未写入 reference_image_ids 的图片"
                    .to_string(),
            ));
        }
        return Ok(());
    }
    let Some(bindings) = bindings else {
        return Err(AppError::InvalidArgument(
            "视频提示词 reference_binding.bindings 不能为空".to_string(),
        ));
    };
    if bindings.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频提示词 reference_binding.bindings 不能为空".to_string(),
        ));
    }

    let allowed = reference_image_ids.iter().copied().collect::<HashSet<_>>();
    let mut covered = HashSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        let object = binding.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 reference_binding.bindings[{}] 必须是对象",
                index + 1
            ))
        })?;
        let image_id = object
            .get("image_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频提示词 reference_binding.bindings[{}].image_id 必须是整数",
                    index + 1
                ))
            })?;
        if image_id <= 0 || !allowed.contains(&image_id) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 reference_binding.bindings[{}].image_id 必须来自 reference_image_ids",
                index + 1
            )));
        }
        validate_structured_value(
            object.get("entity").unwrap_or(&Value::Null),
            &format!(
                "视频提示词 reference_binding.bindings[{}].entity",
                index + 1
            ),
        )?;
        validate_structured_value(
            object.get("role").unwrap_or(&Value::Null),
            &format!("视频提示词 reference_binding.bindings[{}].role", index + 1),
        )?;
        validate_structured_value(
            object.get("usage").unwrap_or(&Value::Null),
            &format!("视频提示词 reference_binding.bindings[{}].usage", index + 1),
        )?;
        validate_structured_value(
            object.get("must_keep").unwrap_or(&Value::Null),
            &format!(
                "视频提示词 reference_binding.bindings[{}].must_keep",
                index + 1
            ),
        )?;
        covered.insert(image_id);
    }

    if covered.len() != allowed.len() {
        let mut missing_ids = allowed.difference(&covered).copied().collect::<Vec<_>>();
        missing_ids.sort_unstable();
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 reference_binding.bindings 必须覆盖所有 reference_image_ids，缺少图片 ID：{}",
            missing_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    Ok(())
}
