use a4n_capabilities::JsonRequest;
use a4n_service_protocol::{SecretForwardingPolicy, TraceContext};
use serde::Serialize;
use serde_json::Value;

use crate::sidecar_clients::{SidecarRequestContext, service_request_with_context, value_as_i64};

use super::context::current_rag_operation_trace_id;

pub(super) fn rag_service_request(
    operation: &str,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    let (project_id, root_project_id) = rag_payload_project_scope(&payload)?;
    let mut context = SidecarRequestContext::current_global()
        .with_project_scope(project_id, root_project_id)
        .with_secret_policy(SecretForwardingPolicy::deny_all());
    if let Some(trace_id) = current_rag_operation_trace_id() {
        context = context.with_trace_context(TraceContext {
            trace_id,
            parent_span_id: Some(operation.to_string()),
        });
    }
    service_request_with_context(
        context,
        a4n_service_protocol::RAG_INDEX_CAPABILITY_VERSION,
        payload,
    )
}

fn rag_payload_project_scope(payload: &Value) -> Result<(Option<i64>, Option<i64>), String> {
    let mut project_ids = Vec::new();
    let mut root_project_ids = Vec::new();
    collect_rag_project_scope_ids(payload, &mut project_ids, &mut root_project_ids)?;
    let project_id = unique_rag_scope_id("project_id", project_ids)?;
    let root_project_id = unique_rag_scope_id("root_project_id", root_project_ids)?;
    if project_id.is_none() && root_project_id.is_some() {
        return Err("RAG payload 包含 root_project_id 但缺少 project_id".to_string());
    }
    Ok((project_id, root_project_id))
}

fn unique_rag_scope_id(field: &str, mut ids: Vec<i64>) -> Result<Option<i64>, String> {
    ids.sort_unstable();
    ids.dedup();
    match ids.as_slice() {
        [] => Ok(None),
        [id] => Ok(Some(*id)),
        ids => Err(format!(
            "单次 RAG capability 请求包含多个 {field}: {}",
            ids.iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn collect_rag_project_scope_ids(
    value: &Value,
    project_ids: &mut Vec<i64>,
    root_project_ids: &mut Vec<i64>,
) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "project_id" || key == "root_project_id" {
                    let id = value_as_i64(value)
                        .ok_or_else(|| format!("RAG payload.{key} 必须是正整数，实际为 {value}"))?;
                    if id <= 0 {
                        return Err(format!("RAG payload.{key} 必须是正整数，实际为 {id}"));
                    }
                    if key == "project_id" {
                        project_ids.push(id);
                    } else {
                        root_project_ids.push(id);
                    }
                } else {
                    collect_rag_project_scope_ids(value, project_ids, root_project_ids)?;
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_rag_project_scope_ids(item, project_ids, root_project_ids)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}
