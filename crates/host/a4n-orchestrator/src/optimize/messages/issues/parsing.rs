use a4n_error::AppError;
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
