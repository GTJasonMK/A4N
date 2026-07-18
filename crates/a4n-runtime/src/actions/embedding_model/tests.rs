use super::{
    contracts::{
        EmbeddingModelDownloadInfoSummary, EmbeddingModelExecution, LocalEmbeddingModelSummary,
    },
    rules::{embedding_model_execution, required_text},
};
use a4n_core::error::AppError;

#[test]
fn blank_model_is_rejected() {
    let err = required_text("  ", "fastembed 模型名").expect_err("blank model should fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn embedding_model_operations_route_only_sidecar_profile_to_capability() {
    assert_eq!(
        embedding_model_execution(crate::state::RuntimeProfile::Sidecar),
        EmbeddingModelExecution::Sidecar
    );
    assert_eq!(
        embedding_model_execution(crate::state::RuntimeProfile::Full),
        EmbeddingModelExecution::HostLocal
    );
    assert_eq!(
        embedding_model_execution(crate::state::RuntimeProfile::Minimal),
        EmbeddingModelExecution::HostLocal
    );
}

#[test]
fn embedding_sidecar_model_payloads_match_host_contracts() {
    let models: Vec<LocalEmbeddingModelSummary> = serde_json::from_value(serde_json::json!([{
        "name": "BGESmallZHV15",
        "model_code": "BAAI/bge-small-zh-v1.5",
        "description": "test",
        "dimension": 512,
        "downloaded": true,
        "cache_path": "/sidecar/cache/model",
        "size_bytes": 42
    }]))
    .expect("embedding sidecar models payload");
    assert_eq!(models[0].dimension, 512);

    let info: EmbeddingModelDownloadInfoSummary = serde_json::from_value(serde_json::json!({
        "model_name": "BGESmallZHV15",
        "model_code": "BAAI/bge-small-zh-v1.5",
        "total_size_bytes": 42,
        "downloaded_size_bytes": 21,
        "missing_size_bytes": 21,
        "files": [{
            "path": "model.onnx",
            "size_bytes": 42,
            "downloaded_size_bytes": 21,
            "downloaded": false
        }]
    }))
    .expect("embedding sidecar download info payload");
    assert_eq!(info.files.len(), 1);
}
