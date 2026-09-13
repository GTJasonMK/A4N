use crate::AppState;
use a4n_api_contract::config::{
    CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput,
};
use a4n_error::AppError;

pub async fn list_embedding_config(
    state: &AppState,
) -> Result<Vec<EmbeddingConfigSummary>, String> {
    state
        .config_read_application()
        .list_embedding()
        .await
        .map_err(|error| error.to_string())
}

pub async fn create_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    input: CreateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, AppError> {
    state
        .config_mutation_application()
        .create_embedding(caller_user_id, input)
        .await
}

pub async fn update_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
    input: UpdateEmbeddingInput,
) -> Result<EmbeddingConfigSummary, AppError> {
    state
        .config_mutation_application()
        .update_embedding(caller_user_id, config_id, input)
        .await
}

pub async fn set_default_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_mutation_application()
        .set_default_embedding(caller_user_id, config_id)
        .await
}

pub async fn delete_embedding_config(
    state: &AppState,
    caller_user_id: i64,
    config_id: i64,
) -> Result<(), AppError> {
    state
        .config_mutation_application()
        .delete_embedding(caller_user_id, config_id)
        .await
}
