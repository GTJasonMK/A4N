use std::path::PathBuf;

use a4n_error::AppError;
use a4n_provider_contract::EmbeddingProviderConfig;

use super::contracts::{positive_usize, required_text};
use super::factory::make_provider_from_default_model;
use super::model_identity_from_config;

fn embedding_model(backend: &str) -> EmbeddingProviderConfig {
    EmbeddingProviderConfig {
        id: 1,
        name: "embedding-test".to_string(),
        backend: backend.to_string(),
        base_url: None,
        api_key: None,
        model: " test-model ".to_string(),
        dimension: 384,
        batch_size: 8,
        local_model_path: None,
    }
}

#[test]
fn dynamic_embedding_requires_database_default_config() {
    let err = match make_provider_from_default_model(None, PathBuf::from("unused")) {
        Ok(_) => panic!("missing default embedding config must fail visibly"),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        AppError::ConfigError(message) if message == "请先设置默认 Embedding 配置"
    ));
}

#[test]
fn embedding_model_identity_uses_shared_backend_and_field_contracts() {
    let identity =
        model_identity_from_config(&embedding_model(" FastEmbed ")).expect("fastembed identity");

    assert_eq!(identity.backend, "FastEmbed");
    assert_eq!(identity.model, "test-model");
    assert_eq!(identity.dimension, 384);
}

#[test]
fn embedding_shared_field_contracts_preserve_errors() {
    assert!(matches!(
        positive_usize(0, "批量大小", "bad"),
        Err(AppError::ConfigError(message)) if message == "嵌入配置 bad 的批量大小必须为正整数"
    ));
    assert!(matches!(
        required_text(Some("  "), "模型名", "bad"),
        Err(AppError::ConfigError(message)) if message == "嵌入配置 bad 缺少模型名"
    ));
}
