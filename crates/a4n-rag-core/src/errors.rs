use std::fmt::Display;

use a4n_core::error::AppError;

pub fn embedding_provider_error(error: impl Display) -> AppError {
    AppError::EmbeddingError(error.to_string())
}

pub fn vector_store_error(error: impl Display) -> AppError {
    AppError::VectorStoreError(error.to_string())
}
