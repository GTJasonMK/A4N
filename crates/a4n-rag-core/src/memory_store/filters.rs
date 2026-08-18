use crate::vector_store::{DeleteFilter, SearchFilter, metadata_matches_search_filter};

pub(super) fn matches_filter(metadata: &serde_json::Value, filter: &SearchFilter) -> bool {
    metadata_matches_search_filter(metadata, filter)
}

pub(super) fn matches_delete_filter(id: &str, filter: &DeleteFilter) -> bool {
    if filter.ids.is_empty() && filter.id_prefixes.is_empty() {
        return true;
    }
    filter.ids.iter().any(|candidate| candidate == id)
        || filter
            .id_prefixes
            .iter()
            .any(|prefix| !prefix.is_empty() && id.starts_with(prefix))
}
