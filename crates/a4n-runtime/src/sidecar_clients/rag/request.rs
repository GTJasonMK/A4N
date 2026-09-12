use a4n_rag_client::JsonRequest;
use a4n_service_protocol::TraceContext;
use serde::Serialize;
use serde_json::Value;

use crate::sidecar_clients::{SidecarRequestContext, service_request_with_value_context};

use super::context::current_rag_operation_trace_id;

pub(super) fn rag_service_request(
    operation: &str,
    payload: impl Serialize,
) -> Result<JsonRequest, String> {
    let payload = serde_json::to_value(payload).map_err(|err| err.to_string())?;
    let project_id = payload
        .get("collection")
        .and_then(Value::as_str)
        .ok_or_else(|| "RAG collection 必须是字符串项目 ID".to_string())?
        .parse::<i64>()
        .map_err(|_| "RAG collection 必须是正整数项目 ID".to_string())?;
    if project_id <= 0 {
        return Err("RAG collection 必须是正整数项目 ID".to_string());
    }
    let mut context = SidecarRequestContext::current_global()
        .with_project_scope(Some(project_id), Some(project_id));
    if let Some(trace_id) = current_rag_operation_trace_id() {
        context = context.with_trace_context(TraceContext {
            trace_id,
            parent_span_id: Some(operation.to_string()),
        });
    }
    service_request_with_value_context(
        context,
        a4n_service_protocol::RAG_INDEX_CAPABILITY_VERSION,
        payload,
    )
}
