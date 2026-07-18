use a4n_core::{
    error::AppError,
    manga::{MANGA_PAGE_DRAWING_PROMPT_V2, MangaPageDrawingPromptV2},
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct PageDrawingPrompt {
    pub(crate) raw: Value,
    pub(crate) page_number: i32,
    pub(crate) image_prompt: String,
}

pub(crate) fn validate_page_drawing_prompts(
    json: &Value,
    context: &str,
) -> Result<Vec<PageDrawingPrompt>, AppError> {
    let pages = json
        .get("pages")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.pages 必须是数组")))?;
    if pages.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.pages 不能为空"
        )));
    }
    pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            let page_context = format!("{context}.pages[{index}]");
            let parsed: MangaPageDrawingPromptV2 =
                serde_json::from_value(page.clone()).map_err(|error| {
                    AppError::InvalidArgument(format!(
                        "{page_context} 不符合 {MANGA_PAGE_DRAWING_PROMPT_V2} 合同: {error}"
                    ))
                })?;
            parsed
                .validate(parsed.page_number)
                .map_err(|error| AppError::InvalidArgument(format!("{page_context}: {error}")))?;
            let page_number = i32::try_from(parsed.page_number).map_err(|_| {
                AppError::InvalidArgument(format!("{page_context}.page_number 超出 i32 范围"))
            })?;
            Ok(PageDrawingPrompt {
                raw: page.clone(),
                page_number,
                image_prompt: parsed.image_prompt,
            })
        })
        .collect()
}

pub(crate) fn page_drawing_prompt_from_response(
    json: &Value,
    expected_page_number: i32,
    context: &str,
) -> Result<Value, AppError> {
    let parsed: MangaPageDrawingPromptV2 =
        serde_json::from_value(json.clone()).map_err(|error| {
            AppError::InvalidArgument(format!(
                "{context} 不符合 {MANGA_PAGE_DRAWING_PROMPT_V2} 合同: {error}"
            ))
        })?;
    let expected_page_number = u32::try_from(expected_page_number).map_err(|_| {
        AppError::InvalidArgument(format!("{context} expected_page_number 必须是正整数"))
    })?;
    parsed
        .validate(expected_page_number)
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    serde_json::to_value(parsed).map_err(AppError::Serialization)
}
