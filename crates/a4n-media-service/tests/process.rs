use std::fs;

use a4n_media_client::{HttpMediaClient, MediaClient};
use a4n_media_contract::{
    MEDIA_FFMPEG_CAPABILITY_VERSION, MediaExecuteInput, MediaFileRef, MediaRoot,
};
use a4n_service_protocol::{ServiceErrorCode, ServiceRequestContext, ServiceStatus, TraceContext};
use a4n_test_support::process::{
    SIDECAR_PROCESS_TEST_TOKEN, SidecarProcessTestDir, assert_protected_route_rejects_invalid_auth,
    assert_route_rejects_malformed_json, assert_route_rejects_unknown_payload_field,
    finish_sidecar, sidecar_base_url, sidecar_manager_with_dir, start_sidecar, unused_local_addr,
};

#[tokio::test]
async fn media_service_process_enforces_local_paths_and_returns_typed_errors() {
    let addr = unused_local_addr();
    let temp_dir = SidecarProcessTestDir::new("media");
    let manager = sidecar_manager_with_dir(
        "media",
        env!("CARGO_BIN_EXE_a4n-media-service"),
        addr,
        "A4N_MEDIA_SERVICE_ADDR",
        &temp_dir,
        [
            (
                "A4N_DATA_DIR".to_string(),
                temp_dir.path().display().to_string(),
            ),
            (
                "A4N_TEMP_DIR".to_string(),
                temp_dir.path().display().to_string(),
            ),
        ],
    );
    fs::write(temp_dir.path().join("note.txt"), b"not a video")
        .expect("write media process fixture");
    let process = start_sidecar(&manager, "media").await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/media/execute").await;
    assert_route_rejects_malformed_json(&base_url, "/media/execute").await;

    let client = HttpMediaClient::new_with_auth_token(base_url.clone(), SIDECAR_PROCESS_TEST_TOKEN);
    let request = ServiceRequestContext::new(
        "media-process-probe",
        TraceContext {
            trace_id: "media-process-probe-trace".to_string(),
            parent_span_id: None,
        },
        a4n_service_protocol::SecretForwardingPolicy::deny_all(),
    )
    .with_payload(
        MEDIA_FFMPEG_CAPABILITY_VERSION,
        MediaExecuteInput::GenerateVideoScrubFrames {
            input: MediaFileRef {
                root: MediaRoot::Data,
                relative_path: "note.txt".to_string(),
            },
            output: MediaFileRef {
                root: MediaRoot::Temp,
                relative_path: "scrub.mjpeg".to_string(),
            },
        },
    )
    .expect("typed media request should be valid");
    assert_route_rejects_unknown_payload_field(&base_url, "/media/execute", &request).await;
    let response = client
        .execute(request)
        .await
        .expect("media worker should return a typed error response");
    assert_eq!(response.status, ServiceStatus::Error);
    let error = response.error.expect("probe error");
    assert_eq!(error.code, ServiceErrorCode::Internal);
    assert!(!error.message.is_empty());

    let traversal = ServiceRequestContext::deny_all(
        "media-process-traversal",
        TraceContext {
            trace_id: "media-process-traversal-trace".to_string(),
            parent_span_id: None,
        },
    )
    .with_payload(
        MEDIA_FFMPEG_CAPABILITY_VERSION,
        MediaExecuteInput::GenerateVideoScrubFrames {
            input: MediaFileRef {
                root: MediaRoot::Data,
                relative_path: "../note.txt".to_string(),
            },
            output: MediaFileRef {
                root: MediaRoot::Temp,
                relative_path: "traversal.mjpeg".to_string(),
            },
        },
    )
    .expect("traversal request envelope should be valid");
    let response = client
        .execute(traversal)
        .await
        .expect("path traversal should return a typed error response");
    assert_eq!(response.status, ServiceStatus::Error);
    assert!(
        response
            .error
            .expect("traversal error")
            .message
            .contains("路径")
    );

    finish_sidecar(process, &manager, "media", addr).await;
}
