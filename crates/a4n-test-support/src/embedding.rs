use a4n_core::error::AppError;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingProvider};
use async_trait::async_trait;

pub struct ShortEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for ShortEmbeddingProvider {
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(vec![vec![1.0, 0.0]])
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "short-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

pub struct ExtraEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for ExtraEmbeddingProvider {
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(vec![vec![1.0, 0.0], vec![0.0, 1.0]])
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "extra-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

pub struct MatchingEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for MatchingEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts
            .iter()
            .enumerate()
            .map(|(index, _)| vec![index as f32, 1.0])
            .collect())
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "matching-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

pub struct WrongDimensionEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for WrongDimensionEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts.into_iter().map(|_| vec![1.0, 0.0]).collect())
    }

    fn dimension(&self) -> usize {
        3
    }

    fn model_name(&self) -> &str {
        "wrong-dimension-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

pub struct UnavailableEmbeddingProvider {
    error: String,
}

impl UnavailableEmbeddingProvider {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[async_trait]
impl EmbeddingProvider for UnavailableEmbeddingProvider {
    async fn embed(&self, _texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Err(AppError::EmbeddingError(self.error.clone()))
    }

    fn dimension(&self) -> usize {
        0
    }

    fn model_name(&self) -> &str {
        "test-unavailable"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

#[cfg(test)]
mod tests;
