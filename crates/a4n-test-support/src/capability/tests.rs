use a4n_capabilities::{
    CapabilityClient, EmbeddingClient, EmbeddingModelOperation, ExportClient, ImageClient,
    LlmClient, RagClient, VideoClient,
};
use a4n_export_core::{ExportRenderJsonInput, ExportRenderZipInput, ExportRenderZipOutput};
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, EMBEDDING_CAPABILITY_VERSION,
    EXPORT_RENDER_CAPABILITY_VERSION, IMAGE_PROVIDER_CAPABILITY_VERSION,
    LLM_CHAT_CAPABILITY_VERSION, RAG_INDEX_CAPABILITY_VERSION, ServiceErrorCode,
    ServiceRequestContext, ServiceResponse, ServiceStatus, TraceContext,
    VIDEO_PROVIDER_CAPABILITY_VERSION,
};
use std::sync::Arc;

use super::{
    PendingEmbeddingDownloadClient, RecordingEmbeddingClient, RecordingExportClient,
    RecordingImageClient, RecordingLlmClient, RecordingRagClient, RecordingVideoClient,
};

#[tokio::test]
async fn recording_llm_probe_returns_configured_response_and_captures_request() {
    let response = response(serde_json::json!({"ok": true}));
    let client = RecordingLlmClient::new(response.clone());
    let request = request("probe-request", LLM_CHAT_CAPABILITY_VERSION);

    let actual = client
        .probe(request.clone())
        .await
        .expect("configured probe response");

    assert_eq!(actual, response);
    assert_eq!(client.probe_call_count(), 1);
    assert_eq!(client.probe_requests(), vec![request.clone()]);
    assert_eq!(client.last_probe_request(), Some(request));
}

#[tokio::test]
async fn recording_llm_unconfigured_operations_fail_explicitly() {
    let client = RecordingLlmClient::new(response(serde_json::Value::Null));
    let health_error = client
        .health()
        .await
        .expect_err("unconfigured health must fail");

    assert_eq!(health_error.code, ServiceErrorCode::Internal);
    assert_eq!(
        health_error.message,
        "unexpected RecordingLlmClient health call"
    );
    assert_eq!(client.probe_call_count(), 0);
}

#[tokio::test]
async fn recording_embedding_returns_configured_response_and_captures_request() {
    let response = response(serde_json::json!({"embeddings": [[1.0, 0.0]]}));
    let client = RecordingEmbeddingClient::new(response.clone());
    let request = request("embed-request", EMBEDDING_CAPABILITY_VERSION);

    let actual = client
        .embed(request.clone())
        .await
        .expect("configured embedding response");

    assert_eq!(actual, response);
    assert_eq!(client.embed_call_count(), 1);
    assert_eq!(client.embed_requests(), vec![request.clone()]);
    assert_eq!(client.last_embed_request(), Some(request));
}

#[tokio::test]
async fn recording_embedding_unconfigured_operations_fail_explicitly() {
    let client = RecordingEmbeddingClient::new(response(serde_json::Value::Null));
    let error = client
        .model_operation(
            EmbeddingModelOperation::List,
            request("model-request", EMBEDDING_CAPABILITY_VERSION),
        )
        .await
        .expect_err("unconfigured model operation must fail");

    assert_eq!(error.code, ServiceErrorCode::Internal);
    assert_eq!(
        error.message,
        "unexpected RecordingEmbeddingClient model_operation call"
    );
    assert_eq!(client.embed_call_count(), 0);
}

#[tokio::test]
async fn pending_embedding_download_observes_start_and_future_drop() {
    let client = Arc::new(PendingEmbeddingDownloadClient::new());
    let observer = client.observer();
    let request = request("embedding-download", EMBEDDING_CAPABILITY_VERSION);
    let task_client = Arc::clone(&client);
    let task_request = request.clone();
    let (events_tx, _events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
    let task = tokio::spawn(async move {
        task_client
            .download_model_events(task_request, events_tx)
            .await
    });

    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        observer.wait_until_started(),
    )
    .await
    .expect("pending download must report that its future started");
    assert_eq!(client.download_model_events_call_count(), 1);
    assert_eq!(client.last_download_model_events_request(), Some(request));

    task.abort();
    let join_error = task
        .await
        .expect_err("aborting the pending download must cancel its task");
    assert!(join_error.is_cancelled());
    assert!(observer.stream_was_dropped());
}

#[tokio::test]
async fn pending_embedding_download_rejects_non_download_operations() {
    let client = PendingEmbeddingDownloadClient::new();
    let error = client
        .embed(request("embedding-pending", EMBEDDING_CAPABILITY_VERSION))
        .await
        .expect_err("pending download client must reject embed");

    assert_eq!(error.code, ServiceErrorCode::Internal);
    assert_eq!(
        error.message,
        "unexpected PendingEmbeddingDownloadClient embed call"
    );
    assert_eq!(client.download_model_events_call_count(), 0);
}

#[tokio::test]
async fn recording_image_generate_events_capture_request_and_bind_task_id() {
    let client = RecordingImageClient::new()
        .with_generate_events(vec![media_event(CapabilityTaskEventKind::Completed, 1)]);
    let request = request("image-generate", IMAGE_PROVIDER_CAPABILITY_VERSION);
    let (events_tx, mut events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);

    client
        .generate_events(request.clone(), events_tx)
        .await
        .expect("configured image events");

    assert_eq!(client.generate_call_count(), 1);
    assert_eq!(client.last_generate_request(), Some(request.clone()));
    let event = events_rx.try_recv().expect("configured completed event");
    assert_eq!(event.task_id, request.request_id);
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
}

#[tokio::test]
async fn recording_image_responses_and_request_ledgers_are_operation_specific() {
    let preview_response = response(serde_json::json!({"operation": "preview"}));
    let client = RecordingImageClient::new().with_preview_response(preview_response.clone());
    let preview_request = request("image-preview", IMAGE_PROVIDER_CAPABILITY_VERSION);

    assert_eq!(
        client
            .preview_request(preview_request.clone())
            .await
            .expect("configured preview response"),
        preview_response
    );
    assert_eq!(client.preview_call_count(), 1);
    assert_eq!(client.generate_call_count(), 0);
    assert_eq!(client.last_preview_request(), Some(preview_request));
}

#[tokio::test]
async fn recording_image_unconfigured_operations_fail_explicitly() {
    let client = RecordingImageClient::new();
    let request = request("image-generate", IMAGE_PROVIDER_CAPABILITY_VERSION);
    let (events_tx, _events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
    let error = client
        .generate_events(request.clone(), events_tx)
        .await
        .expect_err("unconfigured image generate events must fail");

    assert_eq!(error.code, ServiceErrorCode::Internal);
    assert_eq!(
        error.message,
        "unexpected RecordingImageClient generate_events call"
    );
    assert_eq!(client.generate_call_count(), 1);
    assert_eq!(client.last_generate_request(), Some(request));
}

#[tokio::test]
async fn recording_video_generate_events_capture_request_and_bind_task_id() {
    let client = RecordingVideoClient::new()
        .with_generate_events(vec![media_event(CapabilityTaskEventKind::Completed, 1)]);
    let request = request("video-generate", VIDEO_PROVIDER_CAPABILITY_VERSION);
    let (events_tx, mut events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);

    client
        .generate_events(request.clone(), events_tx)
        .await
        .expect("configured video events");

    assert_eq!(client.generate_call_count(), 1);
    assert_eq!(client.last_generate_request(), Some(request.clone()));
    let event = events_rx.try_recv().expect("configured completed event");
    assert_eq!(event.task_id, request.request_id);
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
}

#[tokio::test]
async fn recording_video_responses_and_request_ledgers_are_operation_specific() {
    let poll_response = response(serde_json::json!({"operation": "poll"}));
    let status_response = response(serde_json::json!({"operation": "status"}));
    let preview_response = response(serde_json::json!({"operation": "preview"}));
    let client = RecordingVideoClient::new()
        .with_poll_response(poll_response.clone())
        .with_status_response(status_response.clone())
        .with_preview_response(preview_response.clone());
    let poll_request = request("video-poll", VIDEO_PROVIDER_CAPABILITY_VERSION);
    let status_request = request("video-status", VIDEO_PROVIDER_CAPABILITY_VERSION);
    let preview_request = request("video-preview", VIDEO_PROVIDER_CAPABILITY_VERSION);

    assert_eq!(
        client
            .poll(poll_request.clone())
            .await
            .expect("configured poll response"),
        poll_response
    );
    assert_eq!(
        client
            .status(status_request.clone())
            .await
            .expect("configured status response"),
        status_response
    );
    assert_eq!(
        client
            .preview_request(preview_request.clone())
            .await
            .expect("configured preview response"),
        preview_response
    );
    assert_eq!(client.poll_call_count(), 1);
    assert_eq!(client.status_call_count(), 1);
    assert_eq!(client.preview_call_count(), 1);
    assert_eq!(client.generate_call_count(), 0);
    assert_eq!(client.last_poll_request(), Some(poll_request));
    assert_eq!(client.last_status_request(), Some(status_request));
    assert_eq!(client.last_preview_request(), Some(preview_request));
}

#[tokio::test]
async fn recording_video_unconfigured_operations_fail_explicitly() {
    let client = RecordingVideoClient::new();
    let request = request("video-generate", VIDEO_PROVIDER_CAPABILITY_VERSION);
    let (events_tx, _events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);
    let error = client
        .generate_events(request.clone(), events_tx)
        .await
        .expect_err("unconfigured video generate events must fail");

    assert_eq!(error.code, ServiceErrorCode::Internal);
    assert_eq!(
        error.message,
        "unexpected RecordingVideoClient generate_events call"
    );
    assert_eq!(client.generate_call_count(), 1);
    assert_eq!(client.last_generate_request(), Some(request));
}

#[tokio::test]
async fn recording_rag_ingest_events_capture_request_and_bind_task_id() {
    let client = RecordingRagClient::new()
        .with_ingest_events(vec![media_event(CapabilityTaskEventKind::Completed, 1)]);
    let request = request("rag-ingest", RAG_INDEX_CAPABILITY_VERSION);
    let (events_tx, mut events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);

    client
        .ingest_events(request.clone(), events_tx)
        .await
        .expect("configured RAG ingest events");

    assert_eq!(client.ingest_events_call_count(), 1);
    assert_eq!(client.last_ingest_events_request(), Some(request.clone()));
    let event = events_rx.try_recv().expect("configured completed event");
    assert_eq!(event.task_id, request.request_id);
    assert_eq!(event.event_kind, CapabilityTaskEventKind::Completed);
}

#[tokio::test]
async fn recording_rag_unconfigured_operations_fail_explicitly() {
    let client = RecordingRagClient::new();
    let ingest_request = request("rag-ingest", RAG_INDEX_CAPABILITY_VERSION);
    let (events_tx, _events_rx) =
        tokio::sync::mpsc::channel(a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY);

    let ingest_error = client
        .ingest_events(ingest_request.clone(), events_tx)
        .await
        .expect_err("unconfigured RAG ingest events must fail");
    let query_error = client
        .query(request("rag-query", RAG_INDEX_CAPABILITY_VERSION))
        .await
        .expect_err("unconfigured RAG query must fail");

    assert_eq!(ingest_error.code, ServiceErrorCode::Internal);
    assert_eq!(
        ingest_error.message,
        "unexpected RecordingRagClient ingest_events call"
    );
    assert_eq!(query_error.code, ServiceErrorCode::Internal);
    assert_eq!(
        query_error.message,
        "unexpected RecordingRagClient query call"
    );
    assert_eq!(client.ingest_events_call_count(), 1);
    assert_eq!(client.last_ingest_events_request(), Some(ingest_request));
}

#[tokio::test]
async fn recording_export_zip_response_and_request_ledger_are_typed() {
    let zip_response = response(ExportRenderZipOutput {
        bytes_base64: "UEsDBAo=".to_string(),
        content_type: "application/zip".to_string(),
    });
    let client = RecordingExportClient::new().with_render_zip_response(zip_response.clone());
    let zip_request = export_request(
        "export-zip",
        ExportRenderZipInput {
            manifest_path: "manifest.json".to_string(),
            manifest_bytes_base64: "e30=".to_string(),
            manifest_label: "manifest".to_string(),
            entries: Vec::new(),
            package_label: "package".to_string(),
        },
    );

    assert_eq!(
        client
            .render_zip(zip_request.clone())
            .await
            .expect("configured ZIP response"),
        zip_response
    );
    assert_eq!(client.render_zip_call_count(), 1);
    assert_eq!(client.last_render_zip_request(), Some(zip_request));
}

#[tokio::test]
async fn recording_export_unconfigured_operation_fails_explicitly() {
    let client = RecordingExportClient::new();
    let request = ServiceRequestContext::deny_all(
        "export-json-typed",
        TraceContext {
            trace_id: "trace-export-json-typed".to_string(),
            parent_span_id: None,
        },
    )
    .with_payload(
        EXPORT_RENDER_CAPABILITY_VERSION,
        ExportRenderJsonInput {
            payload: serde_json::Value::Null,
            pretty: false,
        },
    )
    .expect("valid typed export request");

    let error = client
        .render_json(request)
        .await
        .expect_err("unconfigured export must fail");

    assert_eq!(error.code, ServiceErrorCode::Internal);
    assert_eq!(
        error.message,
        "unexpected RecordingExportClient render_json call"
    );
    assert_eq!(client.render_zip_call_count(), 0);
}

fn export_request<T>(request_id: &str, payload: T) -> a4n_service_protocol::ServiceRequest<T> {
    ServiceRequestContext::deny_all(
        request_id,
        TraceContext {
            trace_id: format!("trace-{request_id}"),
            parent_span_id: None,
        },
    )
    .with_payload(EXPORT_RENDER_CAPABILITY_VERSION, payload)
    .expect("valid typed export request")
}

fn request(request_id: &str, capability_version: &str) -> a4n_capabilities::JsonRequest {
    ServiceRequestContext::deny_all(
        request_id,
        TraceContext {
            trace_id: format!("trace-{request_id}"),
            parent_span_id: None,
        },
    )
    .with_payload(
        capability_version,
        serde_json::json!({"request_id": request_id}),
    )
    .expect("valid test capability request")
}

fn response<T>(payload: T) -> ServiceResponse<T> {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some("trace-response".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(payload),
        error: None,
    }
}

fn media_event(event_kind: CapabilityTaskEventKind, sequence: u64) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id: "configured-task".to_string(),
        event_kind,
        sequence,
        error: None,
        payload: None,
    }
}
