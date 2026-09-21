use crate::vector_store::DeleteFilter;

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
