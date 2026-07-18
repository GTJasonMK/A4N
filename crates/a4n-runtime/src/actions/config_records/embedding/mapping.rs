use super::contracts::{CreateEmbeddingInput, EmbeddingConfigSummary, UpdateEmbeddingInput};
use a4n_core::{
    config::{
        CreateEmbeddingConfigParams, UpdateEmbeddingConfigParams, validate_embedding_config_params,
    },
    models::ai::embedding_config,
};
use a4n_orchestrator::config::{
    EmbeddingConfigCreateRecordInput, EmbeddingConfigUpdateRecordInput,
};

pub(super) fn embedding_config_summary(config: &embedding_config::Model) -> EmbeddingConfigSummary {
    EmbeddingConfigSummary {
        id: config.id,
        name: config.name.clone(),
        backend: config.backend.clone(),
        base_url: config.base_url.clone(),
        api_key_encrypted: None,
        has_api_key: config
            .api_key_encrypted
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        model: config.model.clone(),
        dimension: config.dimension,
        batch_size: config.batch_size,
        local_model_path: config.local_model_path.clone(),
        is_default: config.is_default,
        created_at: Some(config.created_at.to_string()),
        updated_at: Some(config.updated_at.to_string()),
    }
}

pub(super) fn embedding_create_record_input(
    input: CreateEmbeddingInput,
) -> Result<EmbeddingConfigCreateRecordInput, String> {
    let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: input.name,
        backend: input.backend,
        base_url: input.base_url,
        api_key_encrypted: input.api_key_encrypted,
        model: input.model,
        dimension: input.dimension,
        batch_size: input.batch_size,
        local_model_path: input.local_model_path,
    })
    .map_err(|err| err.to_string())?;
    Ok(EmbeddingConfigCreateRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
    })
}

pub(super) fn embedding_update_record_input(
    existing: &embedding_config::Model,
    input: UpdateEmbeddingInput,
) -> Result<EmbeddingConfigUpdateRecordInput, String> {
    let params = UpdateEmbeddingConfigParams {
        name: input.name.unwrap_or_else(|| existing.name.clone()),
        backend: input.backend.unwrap_or_else(|| existing.backend.clone()),
        base_url: input.base_url.or_else(|| existing.base_url.clone()),
        api_key_encrypted: input.api_key_encrypted,
        model: input.model.unwrap_or_else(|| existing.model.clone()),
        dimension: input.dimension.unwrap_or(existing.dimension),
        batch_size: input.batch_size.unwrap_or(existing.batch_size),
        local_model_path: input
            .local_model_path
            .or_else(|| existing.local_model_path.clone()),
    };
    let requested_api_key = params
        .api_key_encrypted
        .clone()
        .unwrap_or_else(|| existing.api_key_encrypted.clone());
    let params = validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: requested_api_key,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
    })
    .map_err(|err| err.to_string())?;
    let api_key_encrypted = if params.backend == "api" {
        params.api_key_encrypted
    } else {
        None
    };
    Ok(EmbeddingConfigUpdateRecordInput {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted,
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_embedding_input_requires_batch_size() {
        let err = serde_json::from_value::<CreateEmbeddingInput>(serde_json::json!({
            "name": "missing batch",
            "backend": "api",
            "base_url": "https://api.openai.com/v1",
            "model": "text-embedding-3-small",
            "dimension": 1536
        }))
        .expect_err("batch_size is required by the current create contract");
        assert!(err.to_string().contains("batch_size"));
    }

    #[test]
    fn update_embedding_input_distinguishes_api_key_update_intents() {
        let missing_key = serde_json::from_value::<UpdateEmbeddingInput>(serde_json::json!({
            "name": "keep existing embedding key"
        }))
        .expect("missing api_key_encrypted should preserve the old key");
        assert_eq!(missing_key.api_key_encrypted, None);
        let clear_key = serde_json::from_value::<UpdateEmbeddingInput>(serde_json::json!({
            "api_key_encrypted": null
        }))
        .expect("null api_key_encrypted should be an explicit clear request");
        assert_eq!(clear_key.api_key_encrypted, Some(None));
        let replace_key = serde_json::from_value::<UpdateEmbeddingInput>(serde_json::json!({
            "api_key_encrypted": "sk-new"
        }))
        .expect("string api_key_encrypted should replace the old key");
        assert_eq!(
            replace_key.api_key_encrypted,
            Some(Some("sk-new".to_string()))
        );
    }
}
