use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::Value;

use super::super::OptimizeIssue;

pub(in crate::optimize) fn materialize_optimization_issue_evidence(
    mut issues: Vec<OptimizeIssue>,
    analysis_context: &Value,
) -> Result<Vec<OptimizeIssue>, AppError> {
    let source_spans = analysis_context
        .get("current_chapter")
        .and_then(|value| value.get("source_spans"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("内容优化上下文缺少 current_chapter.source_spans".into())
        })?;
    let source_spans_by_id = source_spans.iter().fold(
        HashMap::with_capacity(source_spans.len()),
        |mut spans_by_id, span| {
            if let Some(id) = span.get("id").and_then(Value::as_str) {
                spans_by_id
                    .entry(id)
                    .or_insert_with(|| span.get("text").and_then(Value::as_str));
            }
            spans_by_id
        },
    );
    for (idx, issue) in issues.iter_mut().enumerate() {
        let arguments = issue.arguments.as_object_mut().ok_or_else(|| {
            AppError::InvalidArgument(format!("issue {idx} arguments 必须是对象"))
        })?;
        if arguments.contains_key("evidence") {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx} arguments.evidence 已废弃；模型必须输出 evidence_span_id"
            )));
        }
        reject_unknown_issue_argument_fields(arguments, idx)?;
        let evidence_span_id = arguments
            .get("evidence_span_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("issue {idx} arguments 缺少 evidence_span_id"))
            })?;
        let evidence =
            resolve_optimization_source_span_text(&source_spans_by_id, &evidence_span_id)
                .map_err(|err| AppError::InvalidArgument(format!("issue {idx} {err}")))?;
        arguments.remove("evidence_span_id");
        arguments.insert("evidence".to_string(), Value::String(evidence));
    }
    Ok(issues)
}

fn reject_unknown_issue_argument_fields(
    arguments: &serde_json::Map<String, Value>,
    idx: usize,
) -> Result<(), AppError> {
    let allowed = ["evidence_span_id", "goal"];
    for key in arguments.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "issue {idx} arguments 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn resolve_optimization_source_span_text(
    source_spans_by_id: &HashMap<&str, Option<&str>>,
    source_span_id: &str,
) -> Result<String, String> {
    source_spans_by_id
        .get(source_span_id)
        .copied()
        .flatten()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            format!("evidence_span_id 不在 current_chapter.source_spans 中: {source_span_id}")
        })
}
