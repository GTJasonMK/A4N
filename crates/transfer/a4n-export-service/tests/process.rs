use a4n_test_support::process::{
    EXPORT_RENDER_CAPABILITY_VERSION, EXPORT_SERVICE_NAME, ExportChapter, ExportClient,
    ExportDocument, ExportDocumentFormat, ExportRenderDocumentInput, ExportRenderJsonInput,
    ExportRenderZipInput, ExportZipEntryBytes, ExportZipPackageInput, HttpExportClient,
    SIDECAR_PROCESS_TEST_TOKEN, ServiceRequestContext, ServiceStatus, TraceContext,
    assert_process_concurrency_saturation, assert_protected_route_rejects_invalid_auth,
    assert_route_rejects_malformed_json, assert_route_rejects_oversized_json,
    assert_route_rejects_unknown_payload_field, assert_route_rejects_version_mismatch,
    disconnect_during_request_body, finish_sidecar, sidecar_base_url, sidecar_manager,
    start_sidecar, unused_local_addr,
};

#[tokio::test]
async fn export_service_process_serves_typed_render_and_stops() {
    let addr = unused_local_addr();
    let (manager, _temp_dir) = sidecar_manager(
        "export",
        env!("CARGO_BIN_EXE_a4n-export-service"),
        addr,
        "A4N_EXPORT_SERVICE_ADDR",
        [],
    );
    let process = start_sidecar(&manager, EXPORT_SERVICE_NAME).await;
    assert_process_concurrency_saturation(addr, EXPORT_SERVICE_NAME, "/export/render_json").await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/export/render_json").await;
    assert_protected_route_rejects_invalid_auth(&base_url, "/export/metrics").await;
    assert_route_rejects_malformed_json(&base_url, "/export/render_json").await;
    assert_route_rejects_oversized_json(&base_url, "/export/render_json", 8 * 1024 * 1024).await;
    assert_route_rejects_oversized_json(&base_url, "/export/render_zip", 64 * 1024 * 1024).await;
    let client = HttpExportClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let request = ServiceRequestContext::new(
        "export-process-render",
        TraceContext {
            trace_id: "export-process-trace".to_string(),
            parent_span_id: None,
        },
        a4n_service_protocol::SecretForwardingPolicy::trusted_remote_no_secrets(),
    )
    .with_payload(
        EXPORT_RENDER_CAPABILITY_VERSION,
        ExportRenderJsonInput {
            payload: serde_json::json!({"value": 1}),
            pretty: false,
        },
    )
    .expect("typed export request should be valid");
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/export/render_json",
        &request,
    )
    .await;
    assert_route_rejects_version_mismatch(&sidecar_base_url(addr), "/export/render_json", &request)
        .await;
    let response = client
        .render_json(request)
        .await
        .expect("typed export render should cross the independent process");
    assert_eq!(response.status, ServiceStatus::Ok);
    assert_eq!(response.payload.expect("export payload"), "{\"value\":1}");

    let document_request = ServiceRequestContext::new(
        "export-process-document",
        TraceContext {
            trace_id: "export-process-document-trace".to_string(),
            parent_span_id: None,
        },
        a4n_service_protocol::SecretForwardingPolicy::trusted_remote_no_secrets(),
    )
    .with_payload(
        EXPORT_RENDER_CAPABILITY_VERSION,
        ExportRenderDocumentInput {
            document: ExportDocument {
                title: "测试作品".to_string(),
                chapters: vec![ExportChapter {
                    chapter_number: 1,
                    title: Some("开端".to_string()),
                    content: "正文".to_string(),
                }],
            },
            format: ExportDocumentFormat::Markdown,
            author: Some("作者".to_string()),
        },
    )
    .expect("typed document render request should be valid");
    let document = client
        .render_document(document_request)
        .await
        .expect("document render should cross the independent process");
    assert_eq!(document.status, ServiceStatus::Ok);
    let document = document.payload.expect("document render payload");
    assert_eq!(document.extension, "md");
    assert_eq!(document.content_type, "text/markdown;charset=utf-8");
    assert!(!document.bytes_base64.is_empty());

    disconnect_during_request_body(addr, "/export/render_zip").await;
    let invalid_zip = client
        .render_zip(
            ServiceRequestContext::deny_all(
                "export-process-invalid-zip",
                TraceContext {
                    trace_id: "export-process-invalid-zip-trace".to_string(),
                    parent_span_id: None,
                },
            )
            .with_payload(
                EXPORT_RENDER_CAPABILITY_VERSION,
                ExportRenderZipInput {
                    manifest_path: "manifest.json".to_string(),
                    manifest_bytes_base64: "not-base64".to_string(),
                    manifest_label:
                        "token=process-export-secret base_url=https://process-export.invalid/v1"
                            .to_string(),
                    entries: Vec::new(),
                    package_label: "process package".to_string(),
                },
            )
            .expect("invalid export zip request envelope should be valid"),
        )
        .await
        .expect("invalid export zip should return a typed response");
    assert_eq!(invalid_zip.status, ServiceStatus::Error);
    let invalid_error = invalid_zip.error.expect("invalid export zip error");
    assert!(!invalid_error.message.contains("process-export-secret"));
    assert!(!invalid_error.message.contains("process-export.invalid"));

    let zip_input = ExportRenderZipInput::from_package_input(ExportZipPackageInput {
        manifest_path: "manifest.json".to_string(),
        manifest_bytes: br#"{"version":"process"}"#.to_vec(),
        manifest_label: "process manifest".to_string(),
        entries: vec![
            ExportZipEntryBytes {
                path: "media/a.txt".to_string(),
                bytes: b"A".to_vec(),
                label: "media a".to_string(),
            },
            ExportZipEntryBytes {
                path: "media/b.txt".to_string(),
                bytes: b"B".to_vec(),
                label: "media b".to_string(),
            },
        ],
        package_label: "process package".to_string(),
    });
    let render_zip_request = |request_id: &str| {
        ServiceRequestContext::deny_all(
            request_id,
            TraceContext {
                trace_id: format!("{request_id}-trace"),
                parent_span_id: None,
            },
        )
        .with_payload(EXPORT_RENDER_CAPABILITY_VERSION, zip_input.clone())
        .expect("typed export zip request should be valid")
    };
    let first_zip = client
        .render_zip(render_zip_request("export-process-zip-first"))
        .await
        .expect("export ZIP should cross the independent process");
    assert_eq!(first_zip.status, ServiceStatus::Ok);
    let first_zip = first_zip.payload.expect("first export ZIP payload");
    assert_eq!(first_zip.content_type, "application/zip");
    let first_bytes = first_zip
        .clone()
        .into_zip_bytes()
        .expect("decode first ZIP");
    assert!(first_bytes.starts_with(b"PK"));

    let second_zip = client
        .render_zip(render_zip_request("export-process-zip-second"))
        .await
        .expect("second export ZIP should cross the independent process");
    assert_eq!(second_zip.status, ServiceStatus::Ok);
    let second_zip = second_zip.payload.expect("second export ZIP payload");
    assert_eq!(first_zip.bytes_base64, second_zip.bytes_base64);

    finish_sidecar(process, &manager, EXPORT_SERVICE_NAME, addr).await;
}
