use a4n_core::error::AppError;
use serde_json::Value;

use super::super::OptimizeIssue;

pub(in crate::optimize) fn parse_optimization_issues(
    raw: &Value,
) -> Result<Vec<OptimizeIssue>, AppError> {
    let arr = raw
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("issues 必须是数组".to_string()))?;
    let mut out = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let object = item
            .as_object()
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 必须是对象")))?;
        reject_unknown_issue_fields(object, idx)?;
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 缺少 id 字段")))?;
        let tool = item
            .get("tool")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.trim().is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 缺少 tool 字段")))?;
        let arguments = item
            .get("arguments")
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument(format!("issue {idx} 缺少 arguments 字段")))?;
        if !arguments.is_object() {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx}.arguments 必须是对象"
            )));
        }
        let description = item
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("issue {idx} 缺少 description 字段"))
            })?;
        let severity = item
            .get("severity")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| matches!(*s, "info" | "minor" | "major" | "critical"))
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "issue {idx}.severity 必须是 info、minor、major 或 critical"
                ))
            })?;
        out.push(OptimizeIssue {
            id,
            tool,
            arguments,
            description,
            severity,
        });
    }
    Ok(out)
}

fn reject_unknown_issue_fields(
    object: &serde_json::Map<String, Value>,
    idx: usize,
) -> Result<(), AppError> {
    let allowed = ["id", "tool", "arguments", "description", "severity"];
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn preview_chars(value: &str, max_chars: usize) -> String {
    let mut preview = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        preview.push_str("...");
    }
    preview
}

pub(in crate::optimize) fn parse_content_optimization_issues(raw: &str) -> Result<Value, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "content_optimization 模型返回空内容，无法解析为 JSON 对象".to_string(),
        ));
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|e| {
        AppError::InvalidArgument(format!(
            "content_optimization 必须返回严格 JSON 对象: {e}; 输出预览：{}",
            preview_chars(trimmed, 240)
        ))
    })?;
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("content_optimization 输出必须是 JSON 对象".to_string())
    })?;
    reject_unknown_optimization_root_fields(object)?;
    let issues = object.get("issues").ok_or_else(|| {
        AppError::InvalidArgument("content_optimization 输出缺少 issues 字段".into())
    })?;
    if !issues.is_array() {
        return Err(AppError::InvalidArgument(
            "content_optimization.issues 必须是数组".to_string(),
        ));
    }
    Ok(issues.clone())
}

fn reject_unknown_optimization_root_fields(
    object: &serde_json::Map<String, Value>,
) -> Result<(), AppError> {
    for key in object.keys() {
        if key != "issues" {
            return Err(AppError::InvalidArgument(format!(
                "content_optimization 输出包含未知字段 {key}；只允许 issues"
            )));
        }
    }
    Ok(())
}
