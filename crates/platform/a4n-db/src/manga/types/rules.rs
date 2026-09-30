use super::errors::{MangaPipelineDbError, MangaPromptRecordError};
use a4n_domain::manga::{MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4};
use serde_json::Value;

pub(in crate::manga) fn require_positive_manga_id(
    value: i64,
    field: &str,
) -> Result<(), MangaPromptRecordError> {
    if value <= 0 {
        return Err(MangaPromptRecordError::InvalidArgument(format!(
            "{field} 必须是正整数"
        )));
    }
    Ok(())
}

pub(in crate::manga) fn require_manga_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, MangaPromptRecordError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(MangaPromptRecordError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}

pub(in crate::manga) fn validate_manga_page_number(
    page_number: i32,
) -> Result<(), MangaPromptRecordError> {
    if page_number <= 0 {
        return Err(MangaPromptRecordError::InvalidArgument(
            "page_number 必须是正整数".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::manga) fn validate_manga_page_prompt_layout(
    page_number: i32,
    layout: Option<&Value>,
) -> Result<(), MangaPipelineDbError> {
    if page_number <= 0 {
        return Err(MangaPipelineDbError::InvalidArgument(
            "page_generation 必须使用 page_number>0".to_string(),
        ));
    }
    let value = layout.cloned().ok_or_else(|| {
        MangaPipelineDbError::InvalidArgument("漫画页面记录必须包含 layout".to_string())
    })?;
    match value.get("schema_version").and_then(Value::as_str) {
        Some("manga_page_generation_v4") => {
            let layout: MangaPageGenerationLayoutV4 =
                serde_json::from_value(value).map_err(|error| {
                    MangaPipelineDbError::InvalidArgument(format!("v4 layout 无效: {error}"))
                })?;
            layout
                .validate(page_number as u32)
                .map_err(|error| MangaPipelineDbError::InvalidArgument(error.to_string()))
        }
        _ => {
            let layout: MangaPageGenerationLayoutV3 =
                serde_json::from_value(value).map_err(|error| {
                    MangaPipelineDbError::InvalidArgument(format!("v3 layout 无效: {error}"))
                })?;
            layout
                .validate(page_number as u32)
                .map_err(|error| MangaPipelineDbError::InvalidArgument(error.to_string()))
        }
    }
}
