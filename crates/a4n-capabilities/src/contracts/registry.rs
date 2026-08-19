use std::sync::Arc;

use super::{
    EmbeddingClient, ExportClient, ImageClient, ImportClient, LlmClient, PromptClient, RagClient,
    VideoClient,
};

#[derive(Clone, Default)]
pub struct A4nCapabilities {
    pub prompt: Option<Arc<dyn PromptClient>>,
    pub llm: Option<Arc<dyn LlmClient>>,
    pub embedding: Option<Arc<dyn EmbeddingClient>>,
    pub rag: Option<Arc<dyn RagClient>>,
    pub image: Option<Arc<dyn ImageClient>>,
    pub video: Option<Arc<dyn VideoClient>>,
    pub import: Option<Arc<dyn ImportClient>>,
    pub export: Option<Arc<dyn ExportClient>>,
}

impl A4nCapabilities {
    pub fn with_prompt(mut self, prompt: Arc<dyn PromptClient>) -> Self {
        self.prompt = Some(prompt);
        self
    }

    pub fn with_llm(mut self, llm: Arc<dyn LlmClient>) -> Self {
        self.llm = Some(llm);
        self
    }

    pub fn with_embedding(mut self, embedding: Arc<dyn EmbeddingClient>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn with_rag(mut self, rag: Arc<dyn RagClient>) -> Self {
        self.rag = Some(rag);
        self
    }

    pub fn with_image(mut self, image: Arc<dyn ImageClient>) -> Self {
        self.image = Some(image);
        self
    }

    pub fn with_video(mut self, video: Arc<dyn VideoClient>) -> Self {
        self.video = Some(video);
        self
    }

    pub fn with_import(mut self, import: Arc<dyn ImportClient>) -> Self {
        self.import = Some(import);
        self
    }

    pub fn with_export(mut self, export: Arc<dyn ExportClient>) -> Self {
        self.export = Some(export);
        self
    }
}
