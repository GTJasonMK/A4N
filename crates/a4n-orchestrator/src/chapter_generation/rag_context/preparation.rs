use a4n_db_models::models::chapter::chapter_outline;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::{
    contracts::sync::ensure_project_embedding_identity_matches,
    search::identity_scoped_search_filter, vector_store::SearchFilter,
};

use super::{
    contracts::PreparedGenerationRagSearch,
    query::build_generation_rag_queries,
    rules::{generation_rag_search_data_types, previous_chapter_range},
};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub(crate) async fn prepare_generation_rag_search(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline: Option<&chapter_outline::Model>,
        outline_text: &str,
        writing_instruction: Option<&str>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<PreparedGenerationRagSearch, String> {
        let pending_foreshadowing = self
            .pending_foreshadowing_before_chapter(project_id, chapter_number, 6)
            .await
            .map_err(|error| error.to_string())?;
        let queries = build_generation_rag_queries(
            outline,
            outline_text,
            writing_instruction,
            &pending_foreshadowing,
        );
        let stored_metadata = self
            .project_rag_metadata(project_id)
            .await
            .map_err(|error| error.to_string())?;
        ensure_project_embedding_identity_matches(&stored_metadata, &embedding_identity)
            .map_err(|error| error.to_string())?;
        let filter =
            build_chapter_generation_rag_search_filter(chapter_number, embedding_identity.clone())?;
        Ok(PreparedGenerationRagSearch {
            project_id,
            chapter_number,
            queries,
            filter,
            embedding_identity,
        })
    }
}

pub(super) fn build_chapter_generation_rag_search_filter(
    chapter_number: i32,
    embedding_identity: EmbeddingModelIdentity,
) -> Result<SearchFilter, String> {
    let mut filter =
        identity_scoped_search_filter(embedding_identity, Some(generation_rag_search_data_types()));
    filter.chapter_range = Some(previous_chapter_range(chapter_number)?);
    Ok(filter)
}
