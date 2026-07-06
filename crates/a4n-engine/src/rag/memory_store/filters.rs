use crate::rag::vector_store::{DeleteFilter, SearchFilter, metadata_matches_search_filter};

pub(super) fn matches_filter(metadata: &serde_json::Value, filter: &SearchFilter) -> bool {
    metadata_matches_search_filter(metadata, filter)
}

pub(super) fn matches_delete_filter(
    id: &str,
    metadata: &serde_json::Value,
    filter: &DeleteFilter,
) -> bool {
    let has_id_filter = !filter.ids.is_empty()
        || filter
            .id_prefixes
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .any(|prefix| !prefix.is_empty());

    if filter.ids.iter().any(|candidate| candidate == id) {
        return true;
    }
    if filter
        .id_prefixes
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|prefix| !prefix.is_empty())
        .any(|prefix| id.starts_with(prefix))
    {
        return true;
    }
    if has_id_filter {
        return false;
    }

    match metadata.get("project_id").and_then(|v| v.as_i64()) {
        Some(project_id) if project_id == filter.project_id => {}
        _ => return false,
    }

    if let Some(data_type) = &filter.data_type {
        match metadata.get("data_type").and_then(|v| v.as_str()) {
            Some(actual) if actual == data_type.as_str() => {}
            _ => return false,
        }
    }

    if let Some(chapter_number) = filter.chapter_number {
        match metadata.get("chapter_number").and_then(|v| v.as_u64()) {
            Some(actual) if actual == u64::from(chapter_number) => {}
            _ => return false,
        }
    }

    true
}
