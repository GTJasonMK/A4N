use std::sync::Arc;

use a4n_capabilities::{
    A4nCapabilities, HttpClientFactory, HttpEmbeddingClient, HttpExportClient, HttpImageClient,
    HttpImportClient, HttpLlmClient, HttpPromptClient, HttpRagClient, HttpVideoClient,
};
use a4n_service_manager::ServiceManager;

use crate::state::RuntimeProfile;

pub(crate) fn runtime_capabilities(
    profile: RuntimeProfile,
    service_manager: &ServiceManager,
) -> Result<A4nCapabilities, String> {
    if !profile.uses_sidecars() {
        return Ok(A4nCapabilities::empty());
    }

    let prompt_url = service_url(service_manager, "prompt")?;
    let llm_url = service_url(service_manager, "llm")?;
    let embedding_url = service_url(service_manager, "embedding")?;
    let rag_url = service_url(service_manager, "rag")?;
    let image_url = service_url(service_manager, "image")?;
    let video_url = service_url(service_manager, "video")?;
    let import_url = service_url(service_manager, "import")?;
    let export_url = service_url(service_manager, "export")?;

    let prompt = http_prompt(service_manager, prompt_url)?;
    let llm = http_llm(service_manager, llm_url)?;
    let embedding = http_embedding(service_manager, embedding_url)?;
    let rag = http_rag(service_manager, rag_url)?;
    let image = http_image(service_manager, image_url)?;
    let video = http_video(service_manager, video_url)?;
    let import_client = http_import(service_manager, import_url)?;
    let export = http_export(service_manager, export_url)?;

    Ok(A4nCapabilities::empty()
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

fn auth_token(manager: &ServiceManager, name: &str) -> Option<String> {
    manager.auth_token(name)
}

fn capability_http_client_factory(
    manager: &ServiceManager,
    name: &str,
) -> Result<HttpClientFactory, String> {
    manager
        .capability_http_client_factory(name)
        .map_err(|error| error.to_string())
}

fn http_prompt(manager: &ServiceManager, url: String) -> Result<HttpPromptClient, String> {
    let capability = match auth_token(manager, "prompt") {
        Some(token) => HttpPromptClient::new_with_auth_token(url, token),
        None => HttpPromptClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "prompt")?))
}

fn http_llm(manager: &ServiceManager, url: String) -> Result<HttpLlmClient, String> {
    let capability = match auth_token(manager, "llm") {
        Some(token) => HttpLlmClient::new_with_auth_token(url, token),
        None => HttpLlmClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "llm")?))
}

fn http_embedding(manager: &ServiceManager, url: String) -> Result<HttpEmbeddingClient, String> {
    let capability = match auth_token(manager, "embedding") {
        Some(token) => HttpEmbeddingClient::new_with_auth_token(url, token),
        None => HttpEmbeddingClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "embedding")?))
}

fn http_rag(manager: &ServiceManager, url: String) -> Result<HttpRagClient, String> {
    let capability = match auth_token(manager, "rag") {
        Some(token) => HttpRagClient::new_with_auth_token(url, token),
        None => HttpRagClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "rag")?))
}

fn http_image(manager: &ServiceManager, url: String) -> Result<HttpImageClient, String> {
    let capability = match auth_token(manager, "image") {
        Some(token) => HttpImageClient::new_with_auth_token(url, token),
        None => HttpImageClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "image")?))
}

fn http_video(manager: &ServiceManager, url: String) -> Result<HttpVideoClient, String> {
    let capability = match auth_token(manager, "video") {
        Some(token) => HttpVideoClient::new_with_auth_token(url, token),
        None => HttpVideoClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "video")?))
}

fn http_import(manager: &ServiceManager, url: String) -> Result<HttpImportClient, String> {
    let capability = match auth_token(manager, "import") {
        Some(token) => HttpImportClient::new_with_auth_token(url, token),
        None => HttpImportClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "import")?))
}

fn http_export(manager: &ServiceManager, url: String) -> Result<HttpExportClient, String> {
    let capability = match auth_token(manager, "export") {
        Some(token) => HttpExportClient::new_with_auth_token(url, token),
        None => HttpExportClient::new(url),
    };
    Ok(capability.with_http_client_factory(capability_http_client_factory(manager, "export")?))
}
