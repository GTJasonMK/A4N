use std::sync::Arc;

use a4n_capabilities::{
    A4nCapabilities, HttpEmbeddingClient, HttpExportClient, HttpImageClient, HttpImportClient,
    HttpLlmClient, HttpPromptClient, HttpRagClient, HttpVideoClient,
};
use a4n_service_manager::{ServiceManager, ServiceMode};
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, IMAGE_SERVICE_NAME, IMPORT_SERVICE_NAME,
    LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, RAG_SERVICE_NAME, VIDEO_SERVICE_NAME,
};

use crate::state::RuntimeProfile;

pub(crate) fn runtime_capabilities(
    profile: RuntimeProfile,
    service_manager: &ServiceManager,
) -> Result<A4nCapabilities, String> {
    if !profile.uses_sidecars() {
        return Ok(A4nCapabilities::default());
    }

    let mut capabilities = A4nCapabilities::default();
    if service_is_configured(service_manager, PROMPT_SERVICE_NAME)? {
        capabilities = capabilities.with_prompt(Arc::new(http_prompt(service_manager)?));
    }
    if service_is_configured(service_manager, LLM_SERVICE_NAME)? {
        capabilities = capabilities.with_llm(Arc::new(http_llm(service_manager)?));
    }
    if service_is_configured(service_manager, EMBEDDING_SERVICE_NAME)? {
        capabilities = capabilities.with_embedding(Arc::new(http_embedding(service_manager)?));
    }
    if service_is_configured(service_manager, RAG_SERVICE_NAME)? {
        capabilities = capabilities.with_rag(Arc::new(http_rag(service_manager)?));
    }
    if service_is_configured(service_manager, IMAGE_SERVICE_NAME)? {
        capabilities = capabilities.with_image(Arc::new(http_image(service_manager)?));
    }
    if service_is_configured(service_manager, VIDEO_SERVICE_NAME)? {
        capabilities = capabilities.with_video(Arc::new(http_video(service_manager)?));
    }
    if service_is_configured(service_manager, IMPORT_SERVICE_NAME)? {
        capabilities = capabilities.with_import(Arc::new(http_import(service_manager)?));
    }
    if service_is_configured(service_manager, EXPORT_SERVICE_NAME)? {
        capabilities = capabilities.with_export(Arc::new(http_export(service_manager)?));
    }
    Ok(capabilities)
}

fn service_is_configured(manager: &ServiceManager, name: &str) -> Result<bool, String> {
    Ok(manager
        .config(name)
        .map_err(|error| error.to_string())?
        .mode
        != ServiceMode::Disabled)
}

fn service_url(manager: &ServiceManager, name: &str) -> Result<String, String> {
    manager
        .config(name)
        .map_err(|error| error.to_string())?
        .url
        .clone()
        .ok_or_else(|| format!("sidecar service {name} 缺少 URL"))
}

macro_rules! http_capability {
    ($function:ident, $client:ty, $service:expr) => {
        fn $function(manager: &ServiceManager) -> Result<$client, String> {
            let url = service_url(manager, $service)?;
            let capability = match manager
                .auth_token($service)
                .map_err(|error| error.to_string())?
            {
                Some(token) => <$client>::new_with_auth_token(url, token),
                None => <$client>::new(url),
            };
            let factory = manager
                .capability_http_client_factory($service)
                .map_err(|error| error.to_string())?;
            Ok(capability.with_http_client_factory(factory))
        }
    };
}

http_capability!(http_prompt, HttpPromptClient, PROMPT_SERVICE_NAME);
http_capability!(http_llm, HttpLlmClient, LLM_SERVICE_NAME);
http_capability!(http_embedding, HttpEmbeddingClient, EMBEDDING_SERVICE_NAME);
http_capability!(http_rag, HttpRagClient, RAG_SERVICE_NAME);
http_capability!(http_image, HttpImageClient, IMAGE_SERVICE_NAME);
http_capability!(http_video, HttpVideoClient, VIDEO_SERVICE_NAME);
http_capability!(http_import, HttpImportClient, IMPORT_SERVICE_NAME);
http_capability!(http_export, HttpExportClient, EXPORT_SERVICE_NAME);
