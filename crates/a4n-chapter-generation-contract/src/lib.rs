//! Runtime-facing application contracts for chapter generation.

use std::sync::Arc;

use a4n_api_contract::{
    chapter::{
        ChapterDerivedCleanupHostPort, PreparedGenerationRagEmbeddingRequest,
        PreviewChapterPromptInput, StartChapterGenerationInput,
    },
    generation_tasks::{GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError},
};
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;

pub struct ChapterGenerationApplicationContext<'a> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub runtime_embedding_identity: Option<EmbeddingModelIdentity>,
}

pub struct ChapterGenerationRequestPreview {
    pub title: String,
    pub note: String,
    pub request: ChatRequest,
}

/// Host-facing boundary for chapter request previews.
#[async_trait::async_trait]
pub trait ChapterGenerationRequestPreviewApplicationPort: Send + Sync {
    async fn preview_request(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterPromptInput,
    ) -> Result<ChapterGenerationRequestPreview, String>;
}

/// Host-facing boundary for chapter-generation RAG embedding preparation.
#[async_trait::async_trait]
pub trait ChapterGenerationRagPreparationApplicationPort: Send + Sync {
    async fn prepare_rag_embedding(
        &self,
        user_id: i64,
        input: PreviewChapterPromptInput,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<PreparedGenerationRagEmbeddingRequest, String>;
}

/// Host-facing boundary for chapter generation workers.
#[async_trait::async_trait]
pub trait ChapterGenerationApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_task(
        &self,
        context: ChapterGenerationApplicationContext<'_>,
        input: StartChapterGenerationInput,
        handle: GenerationTaskHandle,
        host_port: Arc<dyn ChapterDerivedCleanupHostPort>,
    ) -> Result<(), String>;
}
