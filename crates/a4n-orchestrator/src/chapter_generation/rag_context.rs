mod contracts;
mod execution;
mod formatting;
mod fusion;
mod preparation;
mod projection;
mod query;
mod rules;

pub use contracts::{GenerationRagContext, GenerationRagStatistics, PreparedGenerationRagSearch};
pub use fusion::fuse_generation_rag_results;
pub use projection::generation_rag_context_from_results;
pub use query::{GenerationRagQueries, WeightedGenerationQuery, build_generation_rag_queries};
pub use rules::{
    generation_rag_search_data_types, previous_chapter_range, validate_generation_query_vectors,
};

#[cfg(test)]
mod tests;
