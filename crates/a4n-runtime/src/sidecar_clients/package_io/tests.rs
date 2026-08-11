use super::{read_import_media_package_zip, render_export_zip_package};
use crate::state::RuntimeProfile;
use a4n_capabilities::A4nCapabilities;
use a4n_export_core::{ExportRenderZipInput, ExportRenderZipOutput, ExportZipPackageInput};
use a4n_service_protocol::{ServiceResponse, ServiceStatus};
use a4n_test_support::capability::RecordingExportClient;
use serde_json::json;
use std::sync::Arc;

fn zip_package_input() -> ExportZipPackageInput {
    ExportZipPackageInput {
        manifest_path: "manifest.json".to_string(),
        manifest_bytes: br#"{"version":1,"media_files":[]}"#.to_vec(),
        manifest_label: "测试清单".to_string(),
        entries: vec![],
        package_label: "测试项目包".to_string(),
    }
}

#[tokio::test]
async fn import_package_io_sidecar_missing_capability_fails_without_local_fallback() {
    let error = read_import_media_package_zip(
        RuntimeProfile::Sidecar,
        &A4nCapabilities::empty(),
        vec![1, 2, 3],
        "manifest.json",
        "测试项目包",
        "测试清单",
    )
    .await
    .expect_err("missing import capability must fail before local zip parsing");

    assert!(error.contains("import sidecar capability 未配置"));
}

#[tokio::test]
async fn export_package_io_sidecar_missing_capability_fails_without_local_fallback() {
    let error = render_export_zip_package(
        RuntimeProfile::Sidecar,
        &A4nCapabilities::empty(),
        zip_package_input(),
        Some(11),
        Some(3),
    )
    .await
    .expect_err("missing export capability must fail before local zip rendering");

    assert!(error.contains("export sidecar capability 未配置"));
}

#[tokio::test]
async fn embedded_package_io_facade_roundtrips_manifest_and_entries() {
    let capabilities = A4nCapabilities::empty();
    let zip_bytes = render_export_zip_package(
        RuntimeProfile::Minimal,
        &capabilities,
        zip_package_input(),
        Some(11),
        Some(3),
    )
    .await
    .expect("minimal profile must use the local export core through the facade");

    let (manifest, entries) = read_import_media_package_zip(
        RuntimeProfile::Minimal,
        &capabilities,
        zip_bytes,
        "manifest.json",
        "测试项目包",
        "测试清单",
    )
    .await
    .expect("minimal profile must use the local import core through the facade");

    assert_eq!(manifest, json!({"version": 1, "media_files": []}));
    assert!(entries.is_empty());
}

#[tokio::test]
async fn export_sidecar_request_carries_explicit_project_scope() {
    let client = Arc::new(
        RecordingExportClient::new().with_render_zip_response(successful_export_zip_response()),
    );
    let capabilities = A4nCapabilities::empty().with_export(client.clone());

    let zip_bytes = render_export_zip_package(
        RuntimeProfile::Sidecar,
        &capabilities,
        zip_package_input(),
        Some(11),
        Some(3),
    )
    .await
    .expect("sidecar export should render a zip");

    assert!(!zip_bytes.is_empty());
    assert_eq!(client.render_zip_call_count(), 1);
    let request = client
        .last_render_zip_request()
        .expect("recorded export request");
    assert_eq!(request.project_id, Some(11));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.user_id, None);
    assert!(request.schema_version.is_protocol_only());
    assert!(request.secret_policy.is_deny_all());
}

fn successful_export_zip_response() -> ServiceResponse<ExportRenderZipOutput> {
    let input = ExportRenderZipInput::from_package_input(zip_package_input());
    let output = a4n_export_core::render_zip(input).expect("render ZIP output");
    ok_response(output)
}

fn ok_response<T>(payload: T) -> ServiceResponse<T> {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some("test-export".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(payload),
        error: None,
    }
}
