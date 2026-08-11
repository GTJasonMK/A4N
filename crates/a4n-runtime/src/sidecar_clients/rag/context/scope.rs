use std::{
    future::Future,
    time::{SystemTime, UNIX_EPOCH},
};

tokio::task_local! {
    static RAG_OPERATION_TRACE_ID: String;
}

pub(crate) fn scope_rag_operation_trace<F>(
    trace_id: String,
    future: F,
) -> impl Future<Output = F::Output>
where
    F: Future,
{
    RAG_OPERATION_TRACE_ID.scope(trace_id, Box::pin(future))
}

pub(in crate::sidecar_clients::rag) fn current_rag_operation_trace_id() -> Option<String> {
    RAG_OPERATION_TRACE_ID.try_with(Clone::clone).ok()
}

pub(crate) fn rag_operation_timestamp() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| format!("系统时间早于 UNIX epoch，无法生成 RAG operation trace：{error}"))
}
