use super::super::{
    contracts::{FinishChapterGenerationRagContextInput, PreviewChapterPromptInput},
    generation_context::{
        GenerationRagContext, PreparedGenerationRagEmbeddingRequest,
        build_generation_rag_context_with_embeddings, prepare_generation_rag_embedding_request,
    },
};
use crate::{AppState, actions::access::verify_project_editor_string};
use a4n_orchestrator::chapter_generation::{
    merge_generation_outline_text, non_empty_trimmed, required_generation_chapter_title,
};

pub async fn prepare_chapter_generation_rag_embedding(
    state: &AppState,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .orchestrator
        .ensure_text_source_project(input.project_id, "准备章节生成 RAG 检索")
        .await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    if input.chapter_number <= 1 {
        let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
            .await
            .map_err(|error| error.to_string())?;
        return Ok(PreparedGenerationRagEmbeddingRequest {
            query_texts: Vec::new(),
            embedding_identity,
        });
    }
    let outline = state
        .orchestrator
        .required_chapter_generation_outline(input.project_id, input.chapter_number)
        .await?;
    let writing_instruction = input.writing_notes.as_deref().and_then(non_empty_trimmed);
    let chapter_title =
        required_generation_chapter_title(input.chapter_number, Some(&outline), None)?;
    let rag_query_outline_text = merge_generation_outline_text(
        input.chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        None,
    )?;
    prepare_generation_rag_embedding_request(
        state,
        input.project_id,
        input.chapter_number,
        Some(&outline),
        &rag_query_outline_text,
        writing_instruction,
    )
    .await
}

pub async fn finish_chapter_generation_rag_context(
    state: &AppState,
    user_id: i64,
    input: FinishChapterGenerationRagContextInput,
) -> Result<GenerationRagContext, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .orchestrator
        .ensure_text_source_project(input.project_id, "构建章节生成 RAG 上下文")
        .await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    if input.chapter_number <= 1 {
        return Ok(GenerationRagContext::default());
    }
    let outline = state
        .orchestrator
        .required_chapter_generation_outline(input.project_id, input.chapter_number)
        .await?;
    let writing_instruction = input.writing_notes.as_deref().and_then(non_empty_trimmed);
    let chapter_title =
        required_generation_chapter_title(input.chapter_number, Some(&outline), None)?;
    let rag_query_outline_text = merge_generation_outline_text(
        input.chapter_number,
        Some(&outline),
        None,
        writing_instruction,
        Some(&chapter_title),
        None,
    )?;
    build_generation_rag_context_with_embeddings(
        state,
        input.project_id,
        input.chapter_number,
        Some(&outline),
        &rag_query_outline_text,
        writing_instruction,
        input.embedding_identity,
        input.query_vectors,
    )
    .await
}
