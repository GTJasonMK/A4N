use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::OptimizeIssue;
use crate::optimize::runtime_tools::current_chapter_source_markers;

pub(in crate::optimize) fn materialize_optimization_issue_evidence(
    mut issues: Vec<OptimizeIssue>,
    source_context: &Value,
    observed_context: &Value,
) -> Result<Vec<OptimizeIssue>, AppError> {
    let chapter = source_context
        .get("current_chapter")
        .ok_or_else(|| AppError::InvalidArgument("缺少 current_chapter".into()))?;
    let source_markers = current_chapter_source_markers(chapter)?;
    let source_spans = observed_context
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
        let decoded =
            source_markers.decode_output(&json!({"evidence_span_id": evidence_span_id}))?;
        let evidence_span_id = decoded["evidence_span_id"].as_str().ok_or_else(|| {
            AppError::InvalidArgument(format!("issue {idx} evidence_span_id 无效"))
        })?;
        let evidence = resolve_optimization_source_span_text(&source_spans_by_id, evidence_span_id)
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
