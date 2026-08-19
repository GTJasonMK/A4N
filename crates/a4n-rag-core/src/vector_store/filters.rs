use super::contracts::SearchFilter;

pub(crate) fn metadata_matches_search_filter(
    metadata: &serde_json::Value,
    filter: &SearchFilter,
) -> bool {
    if let Some(equals) = &filter.equals {
        for (key, val) in equals {
            if metadata.get(key) != Some(val) {
                return false;
            }
        }
    }
    if let Some(data_types) = &filter.data_types {
        match metadata.get("data_type").and_then(|v| v.as_str()) {
            Some(dt) if data_types.iter().any(|t| t == dt) => {}
            _ => return false,
        }
    }
    if let Some((min, max)) = filter.chapter_range {
        let Some(chapter_number) = metadata.get("chapter_number").and_then(|v| v.as_u64()) else {
            return false;
        };
        let Ok(chapter_number) = u32::try_from(chapter_number) else {
            return false;
        };
        if chapter_number < min || chapter_number > max {
            return false;
        }
    }
    true
}
