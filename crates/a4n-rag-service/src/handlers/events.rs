use crate::app::RagServiceState;
use crate::contracts::{IngestOutput, IngestPayload};
use crate::protocol::{service_error, validate_service_request};
use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceTaskEvent, ServiceTaskEventKind,
    redact_sensitive_message,
};
use a4n_sidecar_bootstrap::{
    service_event_line, service_event_ndjson_response, single_service_event_stream,
};
use axum::{Json, body::Bytes, extract::State, response::IntoResponse};
use futures::{Stream, stream};
use serde_json::{Value, json};
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
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<ServiceTaskEvent>();
    let task_id = request.request_id.clone();
    let record_count = request.payload.records.len();
    tokio::spawn(async move {
        if event_tx
            .send(progress_ingest_event(task_id.clone(), 1, record_count))
            .is_err()
        {
            return;
        }
        let mut ingest_task = tokio::spawn(async move {
            state
                .store
                .add_embeddings(&request.payload.collection, request.payload.records)
                .await
                .map(|written| IngestOutput { written })
        });
        tokio::select! {
            _ = event_tx.closed() => ingest_task.abort(),
            result = &mut ingest_task => {
                let event = match result {
                    Ok(Ok(output)) => match completed_ingest_event(task_id.clone(), 2, output) {
                        Ok(event) => event,
                        Err(error) => failed_ingest_event(task_id, 2, error),
                    },
                    Ok(Err(error)) => failed_ingest_event(task_id, 2, service_error(&error)),
                    Err(error) => failed_ingest_event(
                        task_id,
                        2,
                        ServiceError {
                            code: ServiceErrorCode::Internal,
                            message: redact_sensitive_message(format!(
                                "rag ingest task failed: {error}"
                            )),
                            retryable: Some(false),
                        },
                    ),
                };
                if event_tx.send(event).is_err() {
                    return;
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
) -> ServiceTaskEvent {
    rag_ingest_event(
        task_id,
        ServiceTaskEventKind::Progress,
        sequence,
        None,
        Some(json!({
            "current": 0,
            "total": record_count,
            "message": "rag sidecar 正在写入向量索引"
        })),
    )
}

pub(crate) fn completed_ingest_event(
    task_id: String,
    sequence: u64,
    output: IngestOutput,
) -> Result<ServiceTaskEvent, ServiceError> {
    let payload = serde_json::to_value(output).map_err(|error| ServiceError {
        code: ServiceErrorCode::Internal,
        message: redact_sensitive_message(format!(
            "rag ingest output serialization failed: {error}"
        )),
        retryable: Some(false),
    })?;
    Ok(rag_ingest_event(
        task_id,
        ServiceTaskEventKind::Completed,
        sequence,
        None,
        Some(payload),
    ))
}

pub(crate) fn failed_ingest_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> ServiceTaskEvent {
    rag_ingest_event(
        task_id,
        ServiceTaskEventKind::Failed,
        sequence,
        Some(error),
        None,
    )
}

fn rag_ingest_event(
    task_id: String,
    event_kind: ServiceTaskEventKind,
    sequence: u64,
    error: Option<ServiceError>,
    payload: Option<Value>,
) -> ServiceTaskEvent {
    ServiceTaskEvent {
        task_id,
        event_kind,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error,
        payload,
    }
}
