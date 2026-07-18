use a4n_core::error::AppError;

use a4n_rag_core::vector_store::{
    SearchFilter, SearchResult, VectorStore, metadata_matches_search_filter,
};

pub(super) fn requires_precise_bm25_filter(
    collection: &str,
    filters: Option<&SearchFilter>,
) -> bool {
    let Some(filter) = filters else {
        return false;
    };
    if filter.chapter_range.is_some() {
        return true;
    }
    if filter
        .data_types
        .as_ref()
        .is_some_and(|types| !types.is_empty())
    {
        return true;
    }
    filter.equals.as_ref().is_some_and(|equals| {
        equals.iter().any(|(key, value)| {
            key != "project_id"
                || value
                    .as_i64()
                    .map(|project_id| project_id.to_string() != collection)
                    .unwrap_or(true)
        })
    })
}

pub(super) async fn filter_bm25_results_by_metadata(
    inner: &dyn VectorStore,
    collection: &str,
    bm25_results: Vec<SearchResult>,
    filter: &SearchFilter,
) -> Result<Vec<SearchResult>, AppError> {
    if bm25_results.is_empty() {
        return Ok(Vec::new());
    }

    let ids = bm25_results
        .iter()
        .map(|result| result.id.clone())
        .collect::<Vec<_>>();
    let metadata_by_id = inner.metadata_by_ids(collection, &ids).await?;
    Ok(bm25_results
        .into_iter()
        .filter_map(|mut result| {
            let metadata = metadata_by_id.get(&result.id)?;
            if !metadata_matches_search_filter(metadata, filter) {
                return None;
            }
            result.metadata = metadata.clone();
            Some(result)
        })
        .collect())
}
