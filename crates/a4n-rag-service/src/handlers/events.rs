use crate::app::RagServiceState;
use crate::contracts::{IngestOutput, IngestPayload};
use crate::protocol::{service_error, validate_service_request};
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError, ServiceRequest,
    redact_sensitive_message,
};
use a4n_sidecar_bootstrap::{
    service_event_line, service_event_ndjson_response, single_service_event_stream,
};
use axum::{Json, body::Bytes, extract::State, response::IntoResponse};
use futures::{Stream, stream};
use serde_json::json;
use std::convert::Infallible;

pub(crate) async fn ingest_events(
    State(state): State<RagServiceState>,
    Json(request): Json<ServiceRequest<IngestPayload>>,
) -> impl IntoResponse {
    let task_id = request.request_id.clone();
    if let Err(error) = validate_service_request(&request) {
        return service_event_ndjson_response(
            single_service_event_stream(failed_ingest_event(task_id, 1, error)),
            "rag ingest",
        );
    }
    service_event_ndjson_response(rag_ingest_event_stream(state, request), "rag ingest")
}

fn rag_ingest_event_stream(
    state: RagServiceState,
    request: ServiceRequest<IngestPayload>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<CapabilityTaskEvent>(
        a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY,
    );
    let task_id = request.request_id.clone();
    let record_count = request.payload.records.len();
    tokio::spawn(async move {
        if event_tx
            .send(progress_ingest_event(task_id.clone(), 1, record_count))
            .await
            .is_err()
        {
            return;
        }
        let store = state.store.clone();
        let payload = request.payload;
        let ingest = state.persist_mutation("ingest_events", async move {
            store
                .add_embeddings(&payload.collection, payload.records)
                .await
                .map(|written| IngestOutput { written })
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
                    Ok(output) => match completed_ingest_event(task_id.clone(), 2, output) {
                        Ok(event) => event,
                        Err(error) => failed_ingest_event(task_id, 2, error),
                    },
                    Err(error) => failed_ingest_event(task_id, 2, service_error(&error)),
                };
                if event_tx.send(event).await.is_err() {
                }
            }
        }
    });
    stream::unfold(event_rx, |mut event_rx| async move {
        event_rx
            .recv()
            .await
            .map(|event| (Ok(service_event_line(event)), event_rx))
    })
}

pub(crate) fn progress_ingest_event(
    task_id: String,
    sequence: u64,
    record_count: usize,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent::new(task_id, CapabilityTaskEventKind::Progress, sequence).with_payload(
        json!({
            "current": 0,
            "total": record_count,
            "message": "rag sidecar 正在写入向量索引"
        }),
    )
}

pub(crate) fn completed_ingest_event(
    task_id: String,
    sequence: u64,
    output: IngestOutput,
) -> Result<CapabilityTaskEvent, ServiceError> {
    CapabilityTaskEvent::new(task_id, CapabilityTaskEventKind::Completed, sequence)
        .with_serialized_payload(output, "rag ingest output serialization failed")
}

pub(crate) fn failed_ingest_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent::failed(task_id, sequence, error)
}
