use std::path::Path;

use a4n_core::error::AppError;
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::delete_fastembed_model;
use serde::Deserialize;

use crate::AppState;

use super::super::{
    contracts::{EmbeddingModelExecution, EmbeddingModelInput},
    host_adapter::{embedding_model_execution, required_text},
};

pub async fn delete_local_embedding_model(
    state: &AppState,
    input: EmbeddingModelInput,
) -> Result<(), AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    if embedding_model_execution(state.runtime_profile()) == EmbeddingModelExecution::Sidecar {
        let output: DeleteEmbeddingModelOutput = crate::sidecar_clients::embedding_model_operation(
            state.capabilities(),
            "delete",
            &model,
        )
        .await?;
        if !output.deleted {
            return Err(AppError::Internal(anyhow::anyhow!(
                "embedding sidecar 未确认删除模型: {model}"
            )));
        }
        return Ok(());
    }
    delete_local_embedding_model_in_cache(&model, &state.default_fastembed_cache_dir())
}

#[derive(Debug, Deserialize)]
struct DeleteEmbeddingModelOutput {
    deleted: bool,
}

pub fn delete_local_embedding_model_in_cache(
    model: &str,
    cache_dir: &Path,
) -> Result<(), AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let model = required_text(model, "fastembed 模型名")?;
        delete_fastembed_model(&model, cache_dir)
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (model, cache_dir);
        Err(super::super::provider::local_embedding_disabled_error())
    }
}
