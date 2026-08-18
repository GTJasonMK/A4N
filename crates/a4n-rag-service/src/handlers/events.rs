use crate::app::RagServiceState;
use crate::contracts::IngestPayload;
use crate::protocol::service_error;
use a4n_service_protocol::{
    CapabilityTaskEvent, RAG_SERVICE_NAME, ServiceRequest, redact_sensitive_message,
};
use a4n_sidecar_bootstrap::{
    failed_service_event_response, service_event_channel, service_event_ndjson_response,
};
use axum::{Json, body::Bytes, extract::State, response::IntoResponse};
use futures::Stream;
use std::convert::Infallible;

pub(crate) async fn ingest_events(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<IngestPayload>>,
) -> impl IntoResponse {
    if let Err(error) = request.validate_known_service(RAG_SERVICE_NAME) {
        return failed_service_event_response(request.request_id, error);
    }
    service_event_ndjson_response(rag_ingest_event_stream(state, request))
}

fn rag_ingest_event_stream(
    state: RagServiceState,
    request: ServiceRequest<IngestPayload>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_stream) = service_event_channel();
    let task_id = request.request_id;
    let payload = request.payload;
    tokio::spawn(async move {
        if event_tx
            .send(CapabilityTaskEvent::progress_message(
                task_id.clone(),
                1,
                "rag sidecar 正在写入向量索引",
            ))
            .await
            .is_err()
        {
            return;
        }
        let store = state.store.clone();
        let ingest = state.persist_mutation(async move {
            store
                .add_embeddings(&payload.collection, payload.records)
                .await
        });
        tokio::pin!(ingest);
        tokio::select! {
            _ = event_tx.closed() => {
                if let Err(error) = ingest.await {
                    tracing::error!(error = %redact_sensitive_message(error.to_string()), "rag ingest failed after client disconnect");
                }
            },
            result = &mut ingest => {
                let event = match result {
                    Ok(output) => CapabilityTaskEvent::completed_or_failed(
                        task_id,
                        2,
                        output,
                        "rag ingest output serialization failed",
                    ),
                    Err(error) => CapabilityTaskEvent::failed(task_id, 2, service_error(error)),
                };
                let _ = event_tx.send(event).await;
            }
        }
    });
    event_stream
}
