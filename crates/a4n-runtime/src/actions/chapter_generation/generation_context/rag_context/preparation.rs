use crate::AppState;
use a4n_core::models::chapter::chapter_outline;
use a4n_orchestrator::chapter_generation::PreparedGenerationRagSearch;

pub(super) async fn prepare_generation_rag_search(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
) -> Result<PreparedGenerationRagSearch, String> {
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
    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|err| err.to_string())?;
    state
        .orchestrator
        .prepare_chapter_generation_rag_search(
            project_id,
            chapter_number,
            queries,
            embedding_identity,
        )
        .await
}
