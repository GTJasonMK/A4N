use super::errors::{MangaPipelineDbError, MangaPromptRecordError};
use a4n_domain::manga::MANGA_PAGE_GENERATION_V2;
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
    validate_v2_layout(layout, MANGA_PAGE_GENERATION_V2)
}

fn validate_v2_layout(
    layout: Option<&Value>,
    expected_schema_version: &str,
) -> Result<(), MangaPipelineDbError> {
    let layout = layout.and_then(Value::as_object).ok_or_else(|| {
        MangaPipelineDbError::InvalidArgument(format!(
            "{expected_schema_version} 记录必须包含对象 layout"
        ))
    })?;
    let schema_version = layout
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            MangaPipelineDbError::InvalidArgument(
                "layout.schema_version 必须是非空字符串".to_string(),
            )
        })?;
    if schema_version != expected_schema_version {
        return Err(MangaPipelineDbError::InvalidArgument(format!(
            "layout.schema_version 必须等于 {expected_schema_version}"
        )));
    }
    let shooting_script_id = layout
        .get("shooting_script_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            MangaPipelineDbError::InvalidArgument(
                "layout.shooting_script_id 必须是正整数".to_string(),
            )
        })?;
    let shooting_script_revision = layout
        .get("shooting_script_revision")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            MangaPipelineDbError::InvalidArgument(
                "layout.shooting_script_revision 必须是正整数".to_string(),
            )
        })?;
    if shooting_script_id <= 0 || shooting_script_revision <= 0 {
        return Err(MangaPipelineDbError::InvalidArgument(
            "layout 必须包含正整数 shooting_script_id/shooting_script_revision".to_string(),
        ));
    }
    Ok(())
}
