use a4n_domain::{
    ProviderError as AppError,
    embedding::{EmbeddingBackendKind, embedding_provider_capabilities},
};
use a4n_embedding::http::HttpEmbeddingProvider;
use a4n_embedding::local::LocalEmbeddingProvider;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_embedding_contract::{EmbedTextsOutput, EmbedTextsPayload};
#[cfg(feature = "fastembed")]
use a4n_embedding_fastembed::FastEmbedProvider;
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;

use crate::protocol::{service_error, validate_embedding_request};

pub(crate) async fn embed_texts(
    Json(request): Json<ServiceRequest<EmbedTextsPayload>>,
) -> Json<ServiceResponse<EmbedTextsOutput>> {
    if let Err(error) = validate_embedding_request(&request) {
        return Json(request.error_response(error));
    }
    let trace_id = request.trace_context.trace_id;
    let result = async {
        let provider = embedding_provider(&request.payload)?;
        let (embeddings, identity) = provider.embed_with_identity(request.payload.texts).await?;
        Ok(EmbedTextsOutput {
            identity,
            embeddings,
        })
    }
    .await;
    Json(ServiceResponse::from_result(
        trace_id,
        result,
        service_error,
    ))
}

pub(crate) fn embedding_provider(
    payload: &EmbedTextsPayload,
) -> Result<Box<dyn EmbeddingProvider>, AppError> {
    let model = payload.model.trim();
    if model.is_empty() {
        return Err(AppError::Configuration(
            "embedding sidecar 模型名不能为空".to_string(),
        ));
    }
    if payload.dimension == 0 {
        return Err(AppError::Configuration(
            "embedding sidecar 向量维度必须大于 0".to_string(),
        ));
    }
    if payload.batch_size == 0 {
        return Err(AppError::Configuration(
            "embedding sidecar batch_size 必须大于 0".to_string(),
        ));
    }

    let backend = embedding_provider_capabilities(&payload.backend).ok_or_else(|| {
        AppError::Configuration(format!(
            "embedding sidecar 不支持 backend={}",
            payload.backend.trim()
        ))
    })?;
    let provider: Box<dyn EmbeddingProvider> = match backend.kind {
        #[cfg(feature = "fastembed")]
        EmbeddingBackendKind::FastEmbed => Box::new(FastEmbedProvider::try_new(
            model,
            payload.batch_size,
            crate::handlers::models::resolve_cache_dir()?,
        )?),
        #[cfg(not(feature = "fastembed"))]
        EmbeddingBackendKind::FastEmbed => {
            return Err(AppError::Configuration(
                "embedding sidecar fastembed 支持未编译".to_string(),
            ));
        }
        EmbeddingBackendKind::LocalHash => {
            let expected_model = LocalEmbeddingProvider::new(payload.dimension);
            if model != expected_model.model_name() {
                return Err(AppError::Configuration(format!(
                    "local_hash embedding 模型必须是 {}，实际为 {model}",
                    expected_model.model_name()
                )));
            }
            Box::new(expected_model)
        }
        EmbeddingBackendKind::Api => Box::new(HttpEmbeddingProvider::new_with_batch(
            required_api_base_url(payload)?,
            model,
            payload.dimension,
            payload.api_key.clone(),
            payload.batch_size,
        )),
    };

    if provider.dimension() != payload.dimension {
        return Err(AppError::Configuration(format!(
            "embedding 配置向量维度为 {}，但模型 {} 实际输出 {} 维",
            payload.dimension,
            provider.model_name(),
            provider.dimension()
        )));
    }
    Ok(provider)
}

fn required_api_base_url(payload: &EmbedTextsPayload) -> Result<&str, AppError> {
    payload
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Configuration("API embedding base_url 不能为空".to_string()))
}
