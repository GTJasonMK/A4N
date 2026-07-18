use a4n_core::error::AppError;
use a4n_embedding::local::LocalEmbeddingProvider;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_embedding_fastembed::FastEmbedProvider;
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use crate::contracts::{EmbedTextsOutput, EmbedTextsPayload};
use crate::handlers::models::resolve_cache_dir;
use crate::protocol::{error_response, result_response, validate_service_request};

pub(crate) async fn embed_texts(
    Json(request): Json<ServiceRequest<EmbedTextsPayload>>,
) -> Json<ServiceResponse<EmbedTextsOutput>> {
    if let Err(error) = validate_service_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let provider = embedding_provider(&request.payload)?;
        Ok(EmbedTextsOutput {
            identity: provider.model_identity().await?,
            embeddings: provider.embed(request.payload.texts.clone()).await?,
        })
    }
    .await;
    Json(result_response(&request, result))
}

pub(crate) fn embedding_provider(
    payload: &EmbedTextsPayload,
) -> Result<Box<dyn EmbeddingProvider>, AppError> {
    let model = payload.model.trim();
    if model.is_empty() {
        return Err(AppError::ConfigError(
            "embedding sidecar 模型名不能为空".to_string(),
        ));
    }
    if payload.dimension == 0 {
        return Err(AppError::ConfigError(
            "embedding sidecar 向量维度必须大于 0".to_string(),
        ));
    }

    let provider: Box<dyn EmbeddingProvider> = match payload
        .backend
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "fastembed" => Box::new(FastEmbedProvider::try_new(
            model,
            payload.batch_size,
            resolve_cache_dir()?,
        )?),
        "local_hash" => {
            let expected_model = LocalEmbeddingProvider::new(payload.dimension);
            if model != expected_model.model_name() {
                return Err(AppError::ConfigError(format!(
                    "local_hash embedding 模型必须是 {}，实际为 {model}",
                    expected_model.model_name()
                )));
            }
            Box::new(expected_model)
        }
        "api" => {
            return Err(AppError::ConfigError(
                    "embedding sidecar 不接收 API embedding 后端；API 凭证必须由受信 provider sidecar 单独管理"
                        .to_string(),
                ));
        }
        backend => {
            return Err(AppError::ConfigError(format!(
                "embedding sidecar 不支持 backend={backend}"
            )));
        }
    };

    if provider.dimension() != payload.dimension {
        return Err(AppError::ConfigError(format!(
            "embedding 配置向量维度为 {}，但模型 {} 实际输出 {} 维",
            payload.dimension,
            provider.model_name(),
            provider.dimension()
        )));
    }
    Ok(provider)
}
