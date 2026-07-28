//! 章节生成 runtime 窄 facade。

use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    chapter_content::ChapterDerivedCleanupHostPort,
    chapter_generation::{
        ChapterGenerationApplicationContext, ChapterGenerationRequestPreview,
        FinishChapterGenerationRagContextInput, GenerationRagContext,
        PreparedGenerationRagEmbeddingRequest, PreviewChapterGenerationInput,
    },
};

pub(crate) struct ChapterGenerationRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ChapterGenerationRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn start_chapter_generation_task(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::StartChapterGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_chapter_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_chapter_generation_task(
        &self,
        context: ChapterGenerationApplicationContext<'_>,
        input: a4n_api_contract::chapter::StartChapterGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
        host_port: &dyn ChapterDerivedCleanupHostPort,
    ) -> Result<(), String> {
        self.orchestrator
            .run_chapter_generation_task(context, input, handle, host_port)
            .await
    }

    pub(crate) async fn preview_chapter_generation_request_validated(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterGenerationInput,
    ) -> Result<ChapterGenerationRequestPreview, String> {
        self.orchestrator
            .preview_chapter_generation_request_validated(user_id, context, input)
            .await
    }

    pub(crate) async fn prepare_chapter_generation_rag_embedding_validated(
        &self,
        user_id: i64,
        input: PreviewChapterGenerationInput,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
        self.orchestrator
            .prepare_chapter_generation_rag_embedding_validated(
                user_id,
                input,
                runtime_embedding_identity,
            )
            .await
    }

    pub(crate) async fn finish_chapter_generation_rag_context_validated(
        &self,
        user_id: i64,
        input: FinishChapterGenerationRagContextInput,
    ) -> Result<GenerationRagContext, String> {
        self.orchestrator
            .finish_chapter_generation_rag_context_validated(user_id, input)
            .await
    }

    pub(crate) fn active_generation_chapter(
        &self,
        project_id: i64,
        chapter_numbers: &[i32],
    ) -> Option<i32> {
        self.orchestrator
            .active_generation_chapter(project_id, chapter_numbers)
    }
}
