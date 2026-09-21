#[async_trait::async_trait]
impl ChapterGenerationRequestPreviewApplicationPort for OrchestratorContext {
    async fn preview_request(
        &self,
        user_id: i64,
        context: ChapterGenerationApplicationContext<'_>,
        input: PreviewChapterPromptInput,
    ) -> Result<ChapterGenerationRequestPreview, String> {
        OrchestratorContext::preview_chapter_generation_request_validated(
            self, user_id, context, input,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterGenerationRagPreparationApplicationPort for OrchestratorContext {
    async fn prepare_rag_embedding(
        &self,
        user_id: i64,
        input: PreviewChapterPromptInput,
        runtime_embedding_identity: Option<EmbeddingModelIdentity>,
    ) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
        OrchestratorContext::prepare_chapter_generation_rag_embedding_validated(
            self,
            user_id,
            input,
            runtime_embedding_identity,
        )
        .await
    }
}

#[async_trait::async_trait]
impl ChapterGenerationApplicationPort for OrchestratorContext {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<crate::GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_chapter_generation_task(self, user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_task(
        &self,
        context: ChapterGenerationApplicationContext<'_>,
        input: StartChapterGenerationInput,
        handle: GenerationTaskHandle,
        host_port: Arc<dyn ChapterDerivedCleanupHostPort>,
    ) -> Result<(), String> {
        OrchestratorContext::run_chapter_generation_task(self, context, input, handle, host_port)
            .await
    }
}

fn client_generation_rag_input(
    input: &StartChapterGenerationInput,
) -> Result<Option<ClientGenerationRagInput>, String> {
    let client_rag = match (
        input.client_rag_embedding_identity.as_ref(),
        input.client_rag_query_fingerprint.as_deref(),
        input.client_rag_query_vectors.as_ref(),
    ) {
        (None, None, None) => return Ok(None),
        (Some(identity), Some(query_fingerprint), Some(query_vectors)) => {
            ClientGenerationRagInput {
                embedding_identity: EmbeddingModelIdentity {
                    backend: identity.backend.clone(),
                    model: identity.model.clone(),
                    dimension: identity.dimension,
                },
                query_fingerprint: query_fingerprint.to_string(),
                query_vectors: query_vectors.clone(),
            }
        }
        _ => {
            return Err(
                "章节生成客户端 RAG identity、query fingerprint 和 query vectors 必须同时提供"
                    .to_string(),
            );
        }
    };
    if input.chapter_number <= 1 {
        return Err("第 1 章不接受客户端 RAG 查询向量".to_string());
    }
    Ok(Some(client_rag))
}

async fn fail_chapter_generation_execution(
    orchestrator: &OrchestratorContext,
    chapter_service: &ChapterGenerationService,
    execution: GenerationTaskExecution,
    chapter_session: Option<Arc<ChapterGenerationSession>>,
    workflow_run_id: Option<i64>,
    error: GenerationError,
) -> Result<(), String> {
    if let Some(chapter_session) = chapter_session.as_deref() {
        chapter_service
            .restore_generation_status_ref(chapter_session)
            .await;
    }
    fail_generation_workflow_execution(orchestrator, execution, workflow_run_id, error).await
}

fn check_optional_cancelled(cancel_token: Option<&CancellationToken>) -> Result<(), String> {
    if let Some(cancel_token) = cancel_token {
        check_cancelled(cancel_token)?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), String> {
    cancel_token
        .is_cancelled()
        .then(|| AppError::Cancelled.to_string())
        .map_or(Ok(()), Err)
}
