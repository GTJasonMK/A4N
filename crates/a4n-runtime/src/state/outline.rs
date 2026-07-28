//! 大纲生成领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    outline::{
        ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
        GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
        OutlineGenerationProgressSummary, OutlineRequestPreviewStep, RegenerateChapterOutlineInput,
        RegenerateLastPartInput, RegeneratePartOutlineInput, RegeneratePartsInput,
    },
};

pub(crate) struct OutlineRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> OutlineRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn start_outline_generation_task(
        &self,
        user_id: i64,
        input: a4n_api_contract::outline::StartOutlineGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_outline_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_outline_generation_task(
        &self,
        input: a4n_api_contract::outline::StartOutlineGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_outline_generation_task(input, handle)
            .await
    }

    pub(crate) async fn generation_progress_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, AppError> {
        self.orchestrator
            .outline_generation_progress_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn delete_latest_part_outline_records_validated(
        &self,
        project_id: i64,
        count: u64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_latest_part_outline_records_validated(project_id, count)
            .await
    }

    pub(crate) async fn preview_generate_parts_validated(
        &self,
        input: GeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.orchestrator
            .preview_generate_parts_validated(input)
            .await
    }

    pub(crate) async fn preview_regenerate_parts_validated(
        &self,
        input: RegeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.orchestrator
            .preview_regenerate_parts_validated(input)
            .await
    }

    pub(crate) async fn preview_regenerate_last_part_validated(
        &self,
        input: RegenerateLastPartInput,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .preview_regenerate_last_part_validated(input)
            .await
    }

    pub(crate) async fn preview_continue_part_outlines_validated(
        &self,
        input: ContinuePartOutlinesInput,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .preview_continue_part_outlines_validated(input)
            .await
    }

    pub(crate) async fn preview_regenerate_part_outline_validated(
        &self,
        input: RegeneratePartOutlineInput,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .preview_regenerate_part_outline_validated(input)
            .await
    }

    pub(crate) async fn preview_generate_part_chapter_outlines_validated(
        &self,
        input: GeneratePartChapterOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.orchestrator
            .preview_generate_part_chapter_outlines_validated(input)
            .await
    }

    pub(crate) async fn preview_batch_generate_part_outlines_validated(
        &self,
        input: BatchGeneratePartOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.orchestrator
            .preview_batch_generate_part_outlines_validated(input)
            .await
    }

    pub(crate) async fn preview_generate_chapter_outlines_by_count_validated(
        &self,
        input: GenerateChapterOutlinesByCountInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.orchestrator
            .preview_generate_chapter_outlines_by_count_validated(input)
            .await
    }

    pub(crate) async fn preview_regenerate_chapter_outline_validated(
        &self,
        input: RegenerateChapterOutlineInput,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .preview_regenerate_chapter_outline_validated(input)
            .await
    }

    pub(crate) async fn preview_apply_beat_mapping_validated(
        &self,
        input: ApplyBeatMappingInput,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .preview_apply_beat_mapping_validated(input)
            .await
    }
}
