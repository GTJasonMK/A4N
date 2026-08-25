use std::sync::Arc;

use a4n_api_contract::chapter::StartChapterGenerationInput;
pub use a4n_api_contract::chapter::{
    PreparedGenerationRagEmbeddingRequest, PreviewChapterPromptInput,
};
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
};

pub async fn start_chapter_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartChapterGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let has_client_rag_payload = input.client_rag_embedding_identity.is_some()
        || input.client_rag_query_fingerprint.is_some()
        || input.client_rag_query_vectors.is_some();
    let runtime_embedding_identity = if input.chapter_number > 1 && !has_client_rag_payload {
        runtime_embedding_identity().map_err(GenerationTaskAccessError::InvalidArgument)?
    } else {
        None
    };
    let start = state
        .orchestrator()
        .start_chapter_generation_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    let worker_input = input;
    state.spawn_generation_execution(
        start.handle.clone(),
        "chapter_generation",
        move |handle| async move {
            let prompt_renderer = &worker_state;
            let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(&worker_state);
            worker_state
                .orchestrator()
                .run_chapter_generation_task(
                    ChapterGenerationApplicationContext {
                        prompt_renderer,
                        runtime_embedding_identity,
                    },
                    worker_input,
                    handle,
                    Arc::new(host_port),
                )
                .await
        },
    );
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
    let prompt_renderer = state;
    let preview = state
        .orchestrator()
        .preview_chapter_generation_request_validated(
            user_id,
            ChapterGenerationApplicationContext {
                prompt_renderer,
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
        .orchestrator()
        .prepare_chapter_generation_rag_embedding_validated(
            user_id,
            input,
            runtime_embedding_identity()?,
        )
        .await
}

fn runtime_embedding_identity() -> Result<Option<EmbeddingModelIdentity>, String> {
    crate::runtime_config::current_embedding_model()
        .as_ref()
        .map(crate::dynamic_embedding::model_identity_from_config)
        .transpose()
        .map_err(|error| error.to_string())
}
