use super::{document_export_renderer, read_import_media_package_zip};
use crate::state::RuntimeProfile;
#[cfg(feature = "embedded-transforms")]
use a4n_archive_core::{ArchiveEntry, write_zip};
#[cfg(feature = "embedded-transforms")]
use serde_json::json;

#[tokio::test]
async fn document_parser_requires_sidecar_without_local_fallback() {
    let error = super::import::parse_import_document(
        RuntimeProfile::Sidecar,
        None,
        a4n_import_contract::ImportParseDocumentInput {
            file_type: "txt".into(),
            bytes_base64: "invalid".into(),
            document_title: "作品".into(),
        },
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error, a4n_error::AppError::ConfigError(message) if message.contains("import sidecar capability 未配置"))
    );
}

#[tokio::test]
async fn import_package_io_sidecar_missing_capability_fails_without_local_fallback() {
    let error = read_import_media_package_zip(
        RuntimeProfile::Sidecar,
        None,
        vec![1, 2, 3],
        "manifest.json",
        "测试项目包",
        "测试清单",
    )
    .await
    .expect_err("missing import capability must fail before local zip parsing");

    assert!(error.contains("import sidecar capability 未配置"));
}

#[test]
fn document_export_sidecar_missing_capability_fails_without_local_fallback() {
    let error = document_export_renderer(RuntimeProfile::Sidecar, None)
        .err()
        .expect("missing document export capability must fail");

    assert!(error.contains("export sidecar capability 未配置"));
}

#[cfg(not(feature = "embedded-transforms"))]
#[test]
fn document_export_requires_explicit_embedded_build() {
    assert!(
        document_export_renderer(RuntimeProfile::Minimal, None)
            .err()
            .unwrap()
            .contains("embedded-transforms")
    );
}

#[cfg(feature = "embedded-transforms")]
#[tokio::test]
async fn invalid_document_remains_an_argument_error() {
    let error = super::import::parse_import_document(
        RuntimeProfile::Minimal,
        None,
        a4n_import_contract::ImportParseDocumentInput {
            file_type: "txt".into(),
            bytes_base64: "//8=".into(),
            document_title: "invalid-utf8.txt".into(),
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, a4n_error::AppError::InvalidArgument(_)));
}

#[cfg(feature = "embedded-transforms")]
#[tokio::test]
async fn embedded_package_io_imports_manifest_and_entries() {
    let zip_bytes = write_zip(
        [ArchiveEntry {
            path: "manifest.json".to_string(),
            bytes: br#"{"version":1,"media_files":[]}"#.to_vec(),
            label: "测试清单".to_string(),
        }],
        "测试项目包",
    )
    .expect("test ZIP should be written");

    let (manifest, entries) = read_import_media_package_zip(
        RuntimeProfile::Minimal,
        None,
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
