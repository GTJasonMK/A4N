use a4n_archive_core::{ArchiveEntry, write_zip};
use a4n_service_protocol::BodyLimitKind;
use a4n_test_support::process::{
    HttpImportClient, IMPORT_SERVICE_NAME, IMPORT_TRANSFORM_CAPABILITY_VERSION, ImportClient,
    ImportReadMediaPackageZipInput, SIDECAR_PROCESS_TEST_TOKEN, ServiceRequestContext,
    ServiceStatus, TraceContext, assert_process_concurrency_saturation,
    assert_protected_route_rejects_invalid_auth, assert_route_rejects_malformed_json,
    assert_route_rejects_oversized_json, assert_route_rejects_unknown_payload_field,
    assert_route_rejects_version_mismatch, disconnect_during_request_body, finish_sidecar,
    sidecar_base_url, sidecar_manager, start_sidecar, unused_local_addr,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

fn media_package_input(
    manifest: serde_json::Value,
    entries: &[(&str, &[u8])],
) -> ImportReadMediaPackageZipInput {
    let rendered = write_zip(
        std::iter::once(ArchiveEntry {
            path: "manifest.json".to_string(),
            bytes: serde_json::to_vec(&manifest).expect("serialize import manifest"),
            label: "process manifest".to_string(),
        })
        .chain(entries.iter().map(|(path, bytes)| ArchiveEntry {
            path: (*path).to_string(),
            bytes: bytes.to_vec(),
            label: (*path).to_string(),
        })),
        "process package",
    )
    .map(|bytes| BASE64.encode(bytes))
    .expect("render process import zip");
    ImportReadMediaPackageZipInput {
        zip_bytes_base64: rendered,
        manifest_path: "manifest.json".to_string(),
        package_label: "process package".to_string(),
        manifest_label: "process manifest".to_string(),
    }
}

#[tokio::test]
async fn import_service_process_serves_typed_media_package_and_stops() {
    let addr = unused_local_addr();
    let (manager, _temp_dir) = sidecar_manager(
        "import",
        env!("CARGO_BIN_EXE_a4n-import-service"),
        addr,
        "A4N_IMPORT_SERVICE_ADDR",
        [],
    );
    let process = start_sidecar(&manager, IMPORT_SERVICE_NAME).await;
    assert_process_concurrency_saturation(
        addr,
        IMPORT_SERVICE_NAME,
        "/import/read_media_package_zip",
    )
    .await;
    let base_url = sidecar_base_url(addr);
    assert_protected_route_rejects_invalid_auth(&base_url, "/import/read_media_package_zip").await;
    assert_route_rejects_malformed_json(&base_url, "/import/read_media_package_zip").await;
    assert_route_rejects_oversized_json(
        &base_url,
        "/import/read_media_package_zip",
        IMPORT_SERVICE_NAME,
        BodyLimitKind::Media,
    )
    .await;
    let client = HttpImportClient::new_with_auth_token(base_url, SIDECAR_PROCESS_TEST_TOKEN);
    let request = ServiceRequestContext::new(
        "import-process-media-package",
        TraceContext {
            trace_id: "import-process-media-package-trace".to_string(),
            parent_span_id: None,
        },
        a4n_service_protocol::SecretForwardingPolicy::deny_all(),
    )
    .with_payload(
        IMPORT_TRANSFORM_CAPABILITY_VERSION,
        media_package_input(
            serde_json::json!({
                "version": "process",
                "media_files": [{"package_path": "media/file.txt"}]
            }),
            &[("media/file.txt", b"process media payload")],
        ),
    )
    .expect("typed import request should be valid");
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/import/read_media_package_zip",
        &request,
    )
    .await;
    assert_route_rejects_version_mismatch(
        &sidecar_base_url(addr),
        "/import/read_media_package_zip",
        &request,
    )
    .await;
    let package = client
        .read_media_package_zip(request)
        .await
        .expect("media package import should cross the independent process");
    assert_eq!(package.status, ServiceStatus::Ok);
    let (manifest, entries) = package
        .payload
        .expect("media package import payload")
        .into_manifest_and_entries_bytes()
        .expect("decode imported media entries");
    assert_eq!(manifest["version"], "process");
    assert_eq!(
        entries.get("media/file.txt").map(Vec::as_slice),
        Some(b"process media payload".as_slice())
    );

    // The new operation must work alongside the existing v3 archive endpoint.
    assert_protected_route_rejects_invalid_auth(&sidecar_base_url(addr), "/import/parse_document")
        .await;
    let document_request = ServiceRequestContext::deny_all(
        "import-process-document",
        TraceContext {
            trace_id: "import-process-document-trace".into(),
            parent_span_id: None,
        },
    )
    .with_payload(
        IMPORT_TRANSFORM_CAPABILITY_VERSION,
        a4n_import_contract::ImportParseDocumentInput {
            file_type: "markdown".into(),
                bytes_base64: BASE64.encode("# 第1章 开始\n开篇正文\n\n# 第2章 继续\n后续正文"),
            document_title: "作品".into(),
        },
    )
    .unwrap();
    assert_route_rejects_unknown_payload_field(
        &sidecar_base_url(addr),
        "/import/parse_document",
        &document_request,
    )
    .await;
    let document = client.parse_document(document_request).await.unwrap();
    assert_eq!(document.status, ServiceStatus::Ok);
    let chapters = document.payload.unwrap().chapters;
    assert_eq!(chapters.len(), 2);
    assert_eq!(chapters[0].number, 1);
    assert!(chapters[1].content.contains("后续正文"));

    disconnect_during_request_body(addr, "/import/read_media_package_zip").await;
    let invalid_zip = client
        .read_media_package_zip(
            ServiceRequestContext::deny_all(
                "import-process-invalid-zip",
                TraceContext {
                    trace_id: "import-process-invalid-zip-trace".to_string(),
                    parent_span_id: None,
                },
            )
            .with_payload(
                IMPORT_TRANSFORM_CAPABILITY_VERSION,
                ImportReadMediaPackageZipInput {
                    zip_bytes_base64: "not-base64".to_string(),
                    manifest_path: "manifest.json".to_string(),
                    package_label:
                        "token=process-import-secret base_url=https://process-import.invalid/v1"
                            .to_string(),
                    manifest_label: "process manifest".to_string(),
                },
            )
            .expect("invalid import zip request envelope should be valid"),
        )
        .await
        .expect("invalid import zip should return a typed response");
    assert_eq!(invalid_zip.status, ServiceStatus::Error);
    let invalid_error = invalid_zip.error.expect("invalid import zip error");
    assert!(!invalid_error.message.contains("process-import-secret"));
    assert!(!invalid_error.message.contains("process-import.invalid"));

    let malicious = client
        .read_media_package_zip(
            ServiceRequestContext::deny_all(
                "import-process-malicious-path",
                TraceContext {
                    trace_id: "import-process-malicious-path-trace".to_string(),
                    parent_span_id: None,
                },
            )
            .with_payload(
                IMPORT_TRANSFORM_CAPABILITY_VERSION,
                media_package_input(
                    serde_json::json!({
                        "version": "process",
                        "media_files": [{"package_path": "../secret.txt"}]
                    }),
                    &[],
                ),
            )
            .expect("malicious import request envelope should be valid"),
        )
        .await
        .expect("malicious import path should return a typed response");
    assert_eq!(malicious.status, ServiceStatus::Error);
    assert!(
        malicious
            .error
            .expect("malicious import path error")
            .message
            .contains("不能越界")
    );

    finish_sidecar(process, &manager, IMPORT_SERVICE_NAME, addr).await;
}
