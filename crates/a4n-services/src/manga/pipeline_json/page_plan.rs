use a4n_core::error::AppError;
use serde_json::Value;
use std::collections::HashSet;

use super::fields::{reject_unknown_fields, required_positive_i32_field};

pub(crate) fn page_plan_pages<'a>(json: &'a Value, context: &str) -> Result<&'a [Value], AppError> {
    reject_unknown_fields(json, &["pages"], context)?;
    let pages = json
        .get("pages")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.pages 必须是数组")))?;
    if pages.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.pages 不能为空"
        )));
    }
    let mut seen_page_numbers = HashSet::with_capacity(pages.len());
    for (index, page) in pages.iter().enumerate() {
        let page_context = format!("{context}.pages[{index}]");
        reject_unknown_fields(
            page,
            &[
                "page_number",
                "event_indices",
                "content_summary",
                "content_summary_en",
                "pacing",
                "role",
                "key_characters",
                "has_dialogue",
                "has_action",
                "suggested_panel_count",
                "page_importance",
                "notes",
            ],
            &page_context,
        )?;
        let page_number = page_plan_page_number(page, &page_context)?;
        if !seen_page_numbers.insert(page_number) {
            return Err(AppError::InvalidArgument(format!(
                "{page_context}.page_number 重复：{page_number}"
            )));
        }
        validate_page_event_indices(page, &page_context)?;
    }
    Ok(pages)
}

pub(crate) fn page_plan_page_number(page: &Value, context: &str) -> Result<i32, AppError> {
    required_positive_i32_field(page, "page_number", context)
}

fn validate_page_event_indices(page: &Value, context: &str) -> Result<(), AppError> {
    let values = page
        .get("event_indices")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.event_indices 必须是数组")))?;
    if values.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.event_indices 不能为空"
        )));
    }
    let mut seen = HashSet::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let event_index = value.as_i64().ok_or_else(|| {
            AppError::InvalidArgument(format!("{context}.event_indices[{index}] 必须是非负整数"))
        })?;
        if event_index < 0 {
            return Err(AppError::InvalidArgument(format!(
                "{context}.event_indices[{index}] 必须是非负整数"
            )));
        }
        if !seen.insert(event_index) {
            return Err(AppError::InvalidArgument(format!(
                "{context}.event_indices 包含重复事件索引：{event_index}"
            )));
        }
    }
    Ok(())
}
