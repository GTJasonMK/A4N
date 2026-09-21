use std::fmt::Display;

use a4n_domain::ProviderError as AppError;

pub fn embedding_provider_error(error: impl Display) -> AppError {
    AppError::Embedding(error.to_string())
}

pub fn vector_store_error(error: impl Display) -> AppError {
    AppError::VectorStore(error.to_string())
}
