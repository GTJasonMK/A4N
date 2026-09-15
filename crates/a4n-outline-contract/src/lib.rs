//! Runtime-facing application contracts for outline request previews.
//!
//! The outline owner implements these ports. Runtime only performs Host-side
//! adaptation and consumes the typed preview result.

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError},
    outline::{
        ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
        GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
        RegenerateChapterOutlineInput, RegenerateLastPartInput, RegeneratePartOutlineInput,
        RegeneratePartsInput, StartOutlineGenerationInput,
    },
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

/// One outline LLM request or a reused step that does not issue a request.
#[derive(Debug, Clone)]
pub struct OutlineRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

/// Host-facing boundary for outline generation workers.
///
/// Runtime owns task scheduling; outline validation, LLM workflow state and
/// persistence remain in the outline owner.
#[async_trait::async_trait]
pub trait OutlineGenerationApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_task(
        &self,
        input: StartOutlineGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

/// Host-facing boundary for outline part-generation previews.
#[async_trait::async_trait]
pub trait OutlinePartGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generate_parts(
        &self,
        input: GeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;

    async fn preview_regenerate_parts(
        &self,
        input: RegeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for single-part mutation previews.
#[async_trait::async_trait]
pub trait OutlinePartMutationPreviewApplicationPort: Send + Sync {
    async fn preview_regenerate_last_part(
        &self,
        input: RegenerateLastPartInput,
    ) -> Result<ChatRequest, AppError>;

    async fn preview_continue_part_outlines(
        &self,
        input: ContinuePartOutlinesInput,
    ) -> Result<ChatRequest, AppError>;

    async fn preview_regenerate_part_outline(
        &self,
        input: RegeneratePartOutlineInput,
    ) -> Result<ChatRequest, AppError>;
}

/// Host-facing boundary for generating chapter outlines within one part.
#[async_trait::async_trait]
pub trait OutlinePartChapterGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generate_part_chapter_outlines(
        &self,
        input: GeneratePartChapterOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for batch chapter-outline previews.
#[async_trait::async_trait]
pub trait OutlineBatchPartGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_batch_generate_part_outlines(
        &self,
        input: BatchGeneratePartOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for chapter-count generation previews.
#[async_trait::async_trait]
pub trait OutlineChapterCountGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generate_chapter_outlines_by_count(
        &self,
        input: GenerateChapterOutlinesByCountInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for single chapter-outline regeneration previews.
#[async_trait::async_trait]
pub trait OutlineChapterRegenerationPreviewApplicationPort: Send + Sync {
    async fn preview_regenerate_chapter_outline(
        &self,
        input: RegenerateChapterOutlineInput,
    ) -> Result<ChatRequest, AppError>;
}

/// Host-facing boundary for beat-mapping previews.
#[async_trait::async_trait]
pub trait OutlineBeatMappingPreviewApplicationPort: Send + Sync {
    async fn preview_apply_beat_mapping(
        &self,
        input: ApplyBeatMappingInput,
    ) -> Result<ChatRequest, AppError>;
}
