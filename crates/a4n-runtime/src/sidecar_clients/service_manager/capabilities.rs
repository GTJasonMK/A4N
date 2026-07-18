use std::sync::Arc;

use a4n_capabilities::{
    A4nCapabilities, HttpEmbeddingClient, HttpExportClient, HttpImageClient, HttpImportClient,
    HttpLlmClient, HttpPromptClient, HttpRagClient, HttpVideoClient,
};

use crate::state::RuntimeProfile;

use super::specs::sidecar_url_for_service;

pub(crate) fn runtime_capabilities(profile: RuntimeProfile) -> Result<A4nCapabilities, String> {
    if !profile.uses_sidecars() {
        return Ok(A4nCapabilities::empty());
    }

    Ok(A4nCapabilities::empty()
        .with_prompt(Arc::new(HttpPromptClient::new(sidecar_url_for_service(
            "prompt",
        )?)))
        .with_llm(Arc::new(HttpLlmClient::new(sidecar_url_for_service(
            "llm",
        )?)))
        .with_embedding(Arc::new(HttpEmbeddingClient::new(sidecar_url_for_service(
            "embedding",
        )?)))
        .with_rag(Arc::new(HttpRagClient::new(sidecar_url_for_service(
            "rag",
        )?)))
        .with_image(Arc::new(HttpImageClient::new(sidecar_url_for_service(
            "image",
        )?)))
        .with_video(Arc::new(HttpVideoClient::new(sidecar_url_for_service(
            "video",
        )?)))
        .with_import(Arc::new(HttpImportClient::new(sidecar_url_for_service(
            "import",
        )?)))
        .with_export(Arc::new(HttpExportClient::new(sidecar_url_for_service(
            "export",
        )?))))
}
