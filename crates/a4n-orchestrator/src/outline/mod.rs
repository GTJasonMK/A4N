//! 大纲系统服务：分部大纲 + 章节大纲生成/管理
//!
//! 对应文档 04-大纲系统

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    outline::{
        ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
        GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
        RegenerateChapterOutlineInput, RegenerateLastPartInput, RegeneratePartOutlineInput,
        RegeneratePartsInput, StartOutlineGenerationInput,
    },
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

mod application;
mod chapter_contract;
mod chapters;
mod context;
mod contracts;
mod generation;
mod llm_stream;
mod part_contract;
mod parts;
mod preview;
mod rules;
mod workflow_rules;

pub use contracts::OutlineRequestPreviewStep;
pub(in crate::outline) use contracts::{
    BeatMappingApplyInput, ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, PartOutlineContinuationInput, PartOutlineGenerationInput,
    PartOutlineRegenerationInput, ProgressReporter,
};
pub(in crate::outline) use llm_stream::{
    OutlineLlmStreamContext, execute_outline_llm_call, with_outline_llm_stream,
};
pub(in crate::outline) use preview::{ready_outline_preview_step, reused_outline_preview_step};
pub(in crate::outline) use rules::{
    batch_parts_progress_total, order_parts_for_batch_generation, part_chapter_count,
    progress_total,
};
pub(in crate::outline) use rules::{
    ensure_response_not_truncated, report_progress, validate_existing_chapter_outline_for_request,
};
pub(crate) use workflow_rules::outline_workflow_run_input;

use crate::OrchestratorContext;

#[cfg(test)]
mod tests;

pub(in crate::outline) use chapter_contract::outline_db_error;
pub(crate) use context::{chapter_outline_context_json, outline_formal_materials_context_json};

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
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError>;

    async fn run_task(
        &self,
        input: StartOutlineGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

/// Host-facing boundary for outline beat-mapping previews.
#[async_trait::async_trait]
pub trait OutlineBeatMappingPreviewApplicationPort: Send + Sync {
    async fn preview_apply_beat_mapping(
        &self,
        input: ApplyBeatMappingInput,
    ) -> Result<ChatRequest, AppError>;
}

/// Host-facing boundary for chapter outline regeneration previews.
#[async_trait::async_trait]
pub trait OutlineChapterRegenerationPreviewApplicationPort: Send + Sync {
    async fn preview_regenerate_chapter_outline(
        &self,
        input: RegenerateChapterOutlineInput,
    ) -> Result<ChatRequest, AppError>;
}

/// Host-facing boundary for chapter-count generation previews.
#[async_trait::async_trait]
pub trait OutlineChapterCountGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generate_chapter_outlines_by_count(
        &self,
        input: GenerateChapterOutlinesByCountInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for batch part/chapter outline generation previews.
#[async_trait::async_trait]
pub trait OutlineBatchPartGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_batch_generate_part_outlines(
        &self,
        input: BatchGeneratePartOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for chapter generation within one part previews.
#[async_trait::async_trait]
pub trait OutlinePartChapterGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generate_part_chapter_outlines(
        &self,
        input: GeneratePartChapterOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError>;
}

/// Host-facing boundary for part outline mutation previews.
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

/// Host-facing boundary for part outline generation previews.
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
