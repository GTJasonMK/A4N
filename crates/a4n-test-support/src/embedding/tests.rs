use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingProvider};

use super::{
    ExtraEmbeddingProvider, MatchingEmbeddingProvider, ShortEmbeddingProvider,
    UnavailableEmbeddingProvider, WrongDimensionEmbeddingProvider,
};

#[tokio::test]
async fn short_provider_always_returns_one_embedding() {
    let provider = ShortEmbeddingProvider;
    let embeddings = provider
        .embed(vec!["first".to_string(), "second".to_string()])
        .await
        .expect("short embeddings");

    assert_eq!(embeddings, vec![vec![1.0, 0.0]]);
    assert_eq!(provider.dimension(), 2);
    assert_eq!(provider.model_name(), "short-test");
    assert!(matches!(provider.backend(), EmbeddingBackend::LocalHash));
}

#[tokio::test]
async fn extra_provider_always_returns_two_embeddings() {
    let provider = ExtraEmbeddingProvider;
    let embeddings = provider
        .embed(vec!["only".to_string()])
        .await
        .expect("extra embeddings");

    assert_eq!(embeddings, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    assert_eq!(provider.dimension(), 2);
    assert_eq!(provider.model_name(), "extra-test");
    assert!(matches!(provider.backend(), EmbeddingBackend::LocalHash));
}

#[tokio::test]
async fn matching_provider_returns_one_indexed_embedding_per_input() {
    let provider = MatchingEmbeddingProvider;
    let embeddings = provider
        .embed(vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ])
        .await
        .expect("matching embeddings");

    assert_eq!(
        embeddings,
        vec![vec![0.0, 1.0], vec![1.0, 1.0], vec![2.0, 1.0]]
    );
    assert_eq!(provider.dimension(), 2);
    assert_eq!(provider.model_name(), "matching-test");
    assert!(matches!(provider.backend(), EmbeddingBackend::LocalHash));
}

#[tokio::test]
async fn wrong_dimension_provider_declares_three_but_returns_two_dimensions() {
    let provider = WrongDimensionEmbeddingProvider;
    let embeddings = provider
        .embed(vec!["first".to_string(), "second".to_string()])
        .await
        .expect("wrong-dimension embeddings");

    assert_eq!(embeddings, vec![vec![1.0, 0.0], vec![1.0, 0.0]]);
    assert_eq!(provider.dimension(), 3);
    assert!(embeddings.iter().all(|embedding| embedding.len() == 2));
    assert_eq!(provider.model_name(), "wrong-dimension-test");
    assert!(matches!(provider.backend(), EmbeddingBackend::LocalHash));
}

#[tokio::test]
async fn unavailable_provider_preserves_explicit_failure_contract() {
    let provider = UnavailableEmbeddingProvider::new("embedding is not configured");
    let error = provider
        .embed(vec!["text".to_string()])
        .await
        .expect_err("unavailable embedding must fail");

    assert!(matches!(
        error,
        AppError::EmbeddingError(message) if message == "embedding is not configured"
    ));
    assert_eq!(provider.dimension(), 0);
    assert_eq!(provider.model_name(), "test-unavailable");
    assert!(matches!(provider.backend(), EmbeddingBackend::LocalHash));
}
