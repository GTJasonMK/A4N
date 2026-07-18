use crate::AppState;
use a4n_core::models::chapter::chapter_outline;
use a4n_orchestrator::chapter_generation::GenerationRagContext;

pub(in crate::actions::chapter_generation) async fn build_generation_rag_context(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
) -> Result<GenerationRagContext, String> {
    let queries = state
        .orchestrator
        .prepare_chapter_generation_rag_queries(
            project_id,
            chapter_number,
            outline,
            outline_text,
            writing_instruction,
        )
        .await?;
    if queries.is_empty() {
        return Ok(GenerationRagContext::default());
    }

    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|error| error.to_string())?;
    let prepared = state
        .orchestrator
        .prepare_chapter_generation_rag_search(
            project_id,
            chapter_number,
            queries,
            embedding_identity,
        )
        .await?;
    state
        .orchestrator
        .execute_prepared_chapter_generation_rag_search(&prepared)
        .await
}
