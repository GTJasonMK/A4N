use super::super::types::CharacterAttributeEventHistoryFilter;
use crate::character::{
    CharacterDbError, require_non_empty, validate_category, validate_chapter_number,
};

pub(super) fn validate_operation(operation: &str) -> Result<(), CharacterDbError> {
    if matches!(operation, "add" | "modify" | "delete") {
        Ok(())
    } else {
        Err(CharacterDbError::InvalidArgument(
            "属性事件 operation 必须是 add/modify/delete".to_string(),
        ))
    }
}

pub(super) fn validate_attribute_history_filter(
    filter: &CharacterAttributeEventHistoryFilter,
) -> Result<(), CharacterDbError> {
    if let Some(chapter_from) = filter.chapter_from {
        validate_chapter_number(chapter_from)?;
    }
    if let Some(chapter_to) = filter.chapter_to {
        validate_chapter_number(chapter_to)?;
    }
    if let (Some(chapter_from), Some(chapter_to)) = (filter.chapter_from, filter.chapter_to)
        && chapter_from > chapter_to
    {
        return Err(CharacterDbError::InvalidArgument(
            "属性事件起始章节不能大于结束章节".to_string(),
        ));
    }
    if let Some(category) = filter.category.as_deref() {
        validate_category(category)?;
    }
    if let Some(key) = filter.key.as_deref() {
        require_non_empty(key, "属性 key")?;
    }
    if let Some(operation) = filter.operation.as_deref() {
        validate_operation(operation)?;
    }
    if let Some(limit) = filter.limit
        && !(1..=300).contains(&limit)
    {
        return Err(CharacterDbError::InvalidArgument(
            "属性事件 limit 必须在 1..=300 之间".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_attribute_history_filter;
    use crate::character::{CharacterAttributeEventHistoryFilter, CharacterDbError};

    fn assert_invalid(filter: CharacterAttributeEventHistoryFilter, expected: &str) {
        assert!(matches!(
            validate_attribute_history_filter(&filter),
            Err(CharacterDbError::InvalidArgument(message)) if message == expected
        ));
    }

    #[test]
    fn rejects_invalid_attribute_history_filters() {
        assert_invalid(
            CharacterAttributeEventHistoryFilter {
                chapter_from: Some(5),
                chapter_to: Some(3),
                ..Default::default()
            },
            "属性事件起始章节不能大于结束章节",
        );
        assert_invalid(
            CharacterAttributeEventHistoryFilter {
                category: Some("other".to_string()),
                ..Default::default()
            },
            "属性分类必须是 explicit/implicit/social",
        );
        assert_invalid(
            CharacterAttributeEventHistoryFilter {
                key: Some("  ".to_string()),
                ..Default::default()
            },
            "属性 key不能为空",
        );
        assert_invalid(
            CharacterAttributeEventHistoryFilter {
                operation: Some("replace".to_string()),
                ..Default::default()
            },
            "属性事件 operation 必须是 add/modify/delete",
        );
        assert_invalid(
            CharacterAttributeEventHistoryFilter {
                limit: Some(301),
                ..Default::default()
            },
            "属性事件 limit 必须在 1..=300 之间",
        );
    }
}
