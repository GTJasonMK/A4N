mod application;
mod context_before;
mod lifecycle;
mod preparation;
pub(crate) mod rag_context;

#[path = "chapter_generation/service/mod.rs"]
mod service;

use std::sync::Arc;

use a4n_api_contract::{
    chapter::{
        PreparedGenerationRagEmbeddingRequest, PreviewChapterPromptInput,
        StartChapterGenerationInput,
    },
    generation_tasks::GenerationTaskHandle,
};
use a4n_embedding::provider::EmbeddingModelIdentity;

pub use application::{ChapterGenerationApplicationContext, ChapterGenerationRequestPreview};
pub(crate) use lifecycle::StartChapterGenerationWorkflowInput;
use preparation::{
    ChapterGenerationWordCountRange, compact_json_text, generation_chapter_title,
    is_meaningful_json_value, merge_generation_outline_text, non_empty_trimmed,
    outline_character_names, resolve_generation_word_count_range, truncate_text_chars,
    upsert_chapter_generation_variables,
};
pub(crate) use rag_context::{
    PreparedGenerationRagSearch, generation_rag_query_fingerprint,
    validate_generation_rag_query_fingerprint,
};
pub use service::ChapterGenerationService;

/// Host-facing boundary for chapter request previews.
///
/// Runtime supplies prompt and embedding inputs; chapter preparation and
/// validation stay in this owner.
#[async_trait::async_trait]
pub trait ChapterGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_request(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterPromptInput,
    ) -> Result<ChapterGenerationRequestPreview, String>;

    async fn prepare_rag_embedding(
        &self,
        user_id: i64,
        input: PreviewChapterPromptInput,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<PreparedGenerationRagEmbeddingRequest, String>;
}

/// Host-facing boundary for chapter generation workers.
///
/// Runtime supplies prompt, embedding and cleanup adapters; chapter
/// workflow state and persistence stay in this owner.
#[async_trait::async_trait]
pub trait ChapterGenerationApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError>;

    async fn run_task(
        &self,
        context: ChapterGenerationApplicationContext<'_>,
        input: StartChapterGenerationInput,
        handle: GenerationTaskHandle,
        host_port: Arc<dyn crate::chapter_content::ChapterDerivedCleanupHostPort>,
    ) -> Result<(), String>;
}
