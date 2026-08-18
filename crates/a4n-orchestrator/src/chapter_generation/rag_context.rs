mod contracts;
mod execution;
mod formatting;
mod fusion;
mod preparation;
mod query;
mod rules;

pub(crate) use contracts::PreparedGenerationRagSearch;
pub(crate) use execution::{
    execute_prepared_chapter_generation_rag_search_in_store,
    execute_prepared_chapter_generation_rag_search_with_vectors_in_store,
};
pub(crate) use rules::{
    generation_rag_query_fingerprint, validate_generation_rag_query_fingerprint,
};

#[cfg(test)]
mod tests;
