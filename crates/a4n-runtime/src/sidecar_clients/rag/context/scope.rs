use std::future::Future;

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

pub(super) fn rag_operation_timestamp() -> i64 {
    chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros())
}
