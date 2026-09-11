use super::{
    EmbeddingModelDownloadInfoSummary, LocalEmbeddingModelSummary, host_adapter::required_text,
};
use a4n_error::AppError;

#[test]
fn blank_model_is_rejected() {
    let err = required_text("  ", "fastembed 模型名").expect_err("blank model should fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn embedding_sidecar_model_payloads_match_host_contracts() {
    let models: Vec<LocalEmbeddingModelSummary> = serde_json::from_value(serde_json::json!([{
        "name": "BGESmallZHV15",
        "model_code": "BAAI/bge-small-zh-v1.5",
        "description": "test",
        "dimension": 512,
        "downloaded": true,
        "size_bytes": 42
    }]))
    .expect("embedding sidecar models payload");
    assert_eq!(models[0].dimension, 512);

    let info: EmbeddingModelDownloadInfoSummary = serde_json::from_value(serde_json::json!({
        "model_name": "BGESmallZHV15",
        "model_code": "BAAI/bge-small-zh-v1.5",
        "total_size_bytes": 42,
        "downloaded_size_bytes": 21,
        "missing_size_bytes": 21
    }))
    .expect("embedding sidecar download info payload");
    assert_eq!(info.missing_size_bytes, 21);

    assert!(
        serde_json::from_value::<LocalEmbeddingModelSummary>(serde_json::json!({
            "name": "BGESmallZHV15",
            "model_code": "BAAI/bge-small-zh-v1.5",
            "description": "test",
            "dimension": 512,
            "downloaded": true,
            "cache_path": "/sidecar/cache/model",
            "size_bytes": 42
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<EmbeddingModelDownloadInfoSummary>(serde_json::json!({
            "model_name": "BGESmallZHV15",
            "model_code": "BAAI/bge-small-zh-v1.5",
            "total_size_bytes": 42,
            "downloaded_size_bytes": 21,
            "missing_size_bytes": 21,
            "files": []
        }))
        .is_err()
    );
}
