use a4n_api_contract::chapter::StartChapterGenerationInput;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::GenerationTaskAccessError;
use a4n_orchestrator::chapter_generation::ChapterGenerationApplicationContext;

use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
    prompt_renderer::RuntimePromptRenderer,
};

pub use a4n_orchestrator::chapter_generation::{
    FinishChapterGenerationRagContextInput, GenerationRagContext,
    PreparedGenerationRagEmbeddingRequest,
    PreviewChapterGenerationInput as PreviewChapterPromptInput,
};

pub async fn start_chapter_generation(
    state: &AppState,
    user_id: i64,
    input: StartChapterGenerationInput,
) -> Result<GenerationTaskHandle, String> {
    start_chapter_generation_typed(state, user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn start_chapter_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartChapterGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let runtime_embedding_identity = if input.chapter_number > 1
        && input
            .client_rag_context
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
    {
        runtime_embedding_identity().map_err(GenerationTaskAccessError::InvalidArgument)?
    } else {
        None
    };
    let start = state
        .chapter_generation()
        .start_chapter_generation_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    let handle = start.handle.clone();
    let worker_input = input;
    state.spawn_generation_execution(handle.task_id.clone(), async move {
        let prompt_renderer = RuntimePromptRenderer::new(&worker_state);
        let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(&worker_state);
        if let Err(error) = worker_state
            .chapter_generation()
            .run_chapter_generation_task(
                ChapterGenerationApplicationContext {
                    prompt_renderer: &prompt_renderer,
                    runtime_embedding_identity,
                },
                worker_input,
                handle.clone(),
                &host_port,
            )
            .await
        {
            tracing::error!(
                task_id = %handle.task_id,
                attempt = handle.attempt,
                error = %error,
                "章节 generation worker 失败"
            );
        }
    });
    Ok(start.handle)
}

pub async fn preview_chapter_prompt(
    state: &AppState,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<ModelRequestPreview, String> {
    let runtime_embedding_identity = if input.chapter_number > 1 {
        runtime_embedding_identity()?
    } else {
        None
    };
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let preview = state
        .chapter_generation()
        .preview_chapter_generation_request_validated(
            user_id,
            ChapterGenerationApplicationContext {
                prompt_renderer: &prompt_renderer,
                runtime_embedding_identity,
            },
            input,
        )
        .await?;
    Ok(model_request_preview(
        "chapter_generation",
        preview.title,
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "chapter_generation".to_string(),
                    title: "章节正文生成 LLM 请求".to_string(),
                    prompt_name: Some("writing".to_string()),
                    state: "ready".to_string(),
                    note: Some(preview.note),
                    request: preview.request,
                },
            )
            .await?,
        ],
    ))
}

pub async fn prepare_chapter_generation_rag_embedding(
    state: &AppState,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
    state
        .chapter_generation()
        .prepare_chapter_generation_rag_embedding_validated(
            user_id,
            input,
            runtime_embedding_identity()?,
        )
        .await
}

pub async fn finish_chapter_generation_rag_context(
    state: &AppState,
    user_id: i64,
    input: FinishChapterGenerationRagContextInput,
) -> Result<GenerationRagContext, String> {
    state
        .chapter_generation()
        .finish_chapter_generation_rag_context_validated(user_id, input)
        .await
}

fn runtime_embedding_identity() -> Result<Option<EmbeddingModelIdentity>, String> {
    crate::runtime_config::current_embedding_model()
        .as_ref()
        .map(crate::dynamic_embedding::model_identity_from_config)
        .transpose()
        .map_err(|error| error.to_string())
}
