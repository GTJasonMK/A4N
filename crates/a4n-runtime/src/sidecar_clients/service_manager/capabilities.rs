use std::sync::Arc;

use a4n_embedding_client::{EmbeddingClient, HttpEmbeddingClient};
use a4n_export_client::{ExportClient, HttpExportClient};
use a4n_export_contract::EXPORT_SERVICE_NAME;
use a4n_image_client::{HttpImageClient, ImageClient};
use a4n_import_client::{HttpImportClient, ImportClient};
use a4n_import_contract::IMPORT_SERVICE_NAME;
use a4n_llm_client::{HttpLlmClient, LlmClient};
use a4n_media_client::{HttpMediaClient, MediaClient};
use a4n_media_contract::MEDIA_SERVICE_NAME;
use a4n_prompt_client::{HttpPromptClient, PromptClient};
use a4n_rag_client::{HttpRagClient, RagClient};
use a4n_service_manager::{ServiceManager, ServiceMode};
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, IMAGE_SERVICE_NAME, LLM_SERVICE_NAME, PROMPT_SERVICE_NAME,
    RAG_SERVICE_NAME, VIDEO_SERVICE_NAME,
};
use a4n_video_client::{HttpVideoClient, VideoClient};

use crate::state::RuntimeProfile;

#[derive(Clone, Default)]
pub(crate) struct RuntimeCapabilities {
    pub(crate) prompt: Option<Arc<dyn PromptClient>>,
    pub(crate) llm: Option<Arc<dyn LlmClient>>,
    pub(crate) embedding: Option<Arc<dyn EmbeddingClient>>,
    pub(crate) rag: Option<Arc<dyn RagClient>>,
    pub(crate) image: Option<Arc<dyn ImageClient>>,
    pub(crate) video: Option<Arc<dyn VideoClient>>,
    pub(crate) import: Option<Arc<dyn ImportClient>>,
    pub(crate) export: Option<Arc<dyn ExportClient>>,
    pub(crate) media: Option<Arc<dyn MediaClient>>,
}

impl RuntimeCapabilities {
    fn with_prompt(mut self, prompt: Arc<dyn PromptClient>) -> Self {
        self.prompt = Some(prompt);
        self
    }

    fn with_llm(mut self, llm: Arc<dyn LlmClient>) -> Self {
        self.llm = Some(llm);
        self
    }

    fn with_embedding(mut self, embedding: Arc<dyn EmbeddingClient>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    fn with_rag(mut self, rag: Arc<dyn RagClient>) -> Self {
        self.rag = Some(rag);
        self
    }

    fn with_image(mut self, image: Arc<dyn ImageClient>) -> Self {
        self.image = Some(image);
        self
    }

    fn with_video(mut self, video: Arc<dyn VideoClient>) -> Self {
        self.video = Some(video);
        self
    }

    fn with_import(mut self, import: Arc<dyn ImportClient>) -> Self {
        self.import = Some(import);
        self
    }

    fn with_export(mut self, export: Arc<dyn ExportClient>) -> Self {
        self.export = Some(export);
        self
    }

    fn with_media(mut self, media: Arc<dyn MediaClient>) -> Self {
        self.media = Some(media);
        self
    }
}

pub(crate) fn runtime_capabilities(
    profile: RuntimeProfile,
    service_manager: &ServiceManager,
) -> Result<RuntimeCapabilities, String> {
    if !profile.uses_sidecars() {
        return Ok(RuntimeCapabilities::default());
    }

    let mut capabilities = RuntimeCapabilities::default();
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
    if service_is_configured(service_manager, MEDIA_SERVICE_NAME)? {
        capabilities = capabilities.with_media(Arc::new(http_media(service_manager)?));
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
http_capability!(http_media, HttpMediaClient, MEDIA_SERVICE_NAME);
