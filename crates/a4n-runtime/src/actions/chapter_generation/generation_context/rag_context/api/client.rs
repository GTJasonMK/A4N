use super::super::{
    contracts::PreparedGenerationRagEmbeddingRequest, preparation::prepare_generation_rag_search,
};
use crate::AppState;
use a4n_core::models::chapter::chapter_outline;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::chapter_generation::GenerationRagContext;

pub(in crate::actions::chapter_generation) async fn prepare_generation_rag_embedding_request(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
    let prepared = prepare_generation_rag_search(
        state,
        project_id,
        chapter_number,
        outline,
        outline_text,
        writing_instruction,
    )
    .await?;
    Ok(PreparedGenerationRagEmbeddingRequest {
        query_texts: prepared
            .queries
            .weighted_queries()
            .into_iter()
            .map(|query| query.text.to_string())
            .collect(),
        embedding_identity: prepared.embedding_identity,
    })
}

pub(in crate::actions::chapter_generation) async fn build_generation_rag_context_with_embeddings(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
    embedding_identity: EmbeddingModelIdentity,
    query_vectors: Vec<Vec<f32>>,
) -> Result<GenerationRagContext, String> {
    let prepared = prepare_generation_rag_search(
        state,
        project_id,
        chapter_number,
        outline,
        outline_text,
        writing_instruction,
    )
    .await?;
    state
        .orchestrator
        .execute_prepared_chapter_generation_rag_search_with_vectors(
            &prepared,
            &embedding_identity,
            query_vectors,
        )
        .await
}
