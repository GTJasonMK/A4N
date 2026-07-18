use super::{
    contracts::{GenerationRagContext, GenerationRagStatistics},
    formatting::format_generation_rag_context,
    query::GenerationRagQueries,
    rules::context_text_length,
};

pub fn generation_rag_context_from_results(
    results: Vec<a4n_rag_core::vector_store::SearchResult>,
    queries: &GenerationRagQueries,
    project_id: i64,
    chapter_number: i32,
) -> GenerationRagContext {
    let formatted = format_generation_rag_context(&results, project_id, chapter_number);
    let statistics = GenerationRagStatistics {
        chunk_count: formatted.chunk_count,
        summary_count: formatted.summary_count,
        continuity_count: formatted.continuity_count,
        context_length: context_text_length(formatted.text.as_deref()),
        query_main: Some(queries.main_query.clone()),
        query_characters: queries.character_queries.clone(),
        query_foreshadowing: queries.foreshadowing_queries.clone(),
        query_location: queries.location_query.clone(),
    };
    GenerationRagContext {
        text: formatted.text,
        statistics,
    }
}
