use std::sync::Arc;

use a4n_capabilities::{
    A4nCapabilities, HttpEmbeddingClient, HttpExportClient, HttpImageClient, HttpImportClient,
    HttpLlmClient, HttpPromptClient, HttpRagClient, HttpVideoClient,
};
use a4n_service_manager::ServiceManager;
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

    let prompt = http_prompt(service_manager)?;
    let llm = http_llm(service_manager)?;
    let embedding = http_embedding(service_manager)?;
    let rag = http_rag(service_manager)?;
    let image = http_image(service_manager)?;
    let video = http_video(service_manager)?;
    let import_client = http_import(service_manager)?;
    let export = http_export(service_manager)?;

    Ok(A4nCapabilities::default()
        .with_prompt(Arc::new(prompt))
        .with_llm(Arc::new(llm))
        .with_embedding(Arc::new(embedding))
        .with_rag(Arc::new(rag))
        .with_image(Arc::new(image))
        .with_video(Arc::new(video))
        .with_import(Arc::new(import_client))
        .with_export(Arc::new(export)))
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
