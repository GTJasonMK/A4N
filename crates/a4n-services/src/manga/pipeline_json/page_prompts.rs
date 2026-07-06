use a4n_core::error::AppError;
use serde_json::Value;

use super::fields::{
    optional_string_field, reject_unknown_fields, required_non_empty_string_field,
    required_positive_i32_field,
};

pub(crate) struct PageDrawingPrompt<'a> {
    pub(crate) raw: &'a Value,
    pub(crate) page_number: i32,
    pub(crate) image_prompt: &'a str,
    pub(crate) dialogue_text: Option<&'a str>,
}

pub(crate) fn page_drawing_prompt_pages<'a>(
    json: &'a Value,
    context: &str,
) -> Result<&'a [Value], AppError> {
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
    Ok(pages)
}

pub(crate) fn validate_page_drawing_prompts<'a>(
    json: &'a Value,
    context: &str,
) -> Result<Vec<PageDrawingPrompt<'a>>, AppError> {
    page_drawing_prompt_pages(json, context)?
        .iter()
        .enumerate()
        .map(|(index, page)| {
            let page_context = format!("{context}.pages[{index}]");
            reject_unknown_fields(
                page,
                &["page_number", "image_prompt", "dialogue_text"],
                &page_context,
            )?;
            Ok(PageDrawingPrompt {
                raw: page,
                page_number: required_positive_i32_field(page, "page_number", &page_context)?,
                image_prompt: required_non_empty_string_field(page, "image_prompt", &page_context)?,
                dialogue_text: optional_string_field(page, "dialogue_text", &page_context)?,
            })
        })
        .collect()
}

pub(crate) fn page_drawing_prompt_from_response(
    json: &Value,
    expected_page_number: i32,
    context: &str,
) -> Result<Value, AppError> {
    let pages = validate_page_drawing_prompts(json, context)?;
    if pages.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context}.pages 必须只包含第 {expected_page_number} 页"
        )));
    }
    let page = &pages[0];
    if page.page_number != expected_page_number {
        return Err(AppError::InvalidArgument(format!(
            "{context}.pages[0].page_number 应为 {expected_page_number}，实际为 {}",
            page.page_number
        )));
    }
    Ok(page.raw.clone())
}
