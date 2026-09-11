use a4n_domain::manga::{MANGA_PAGE_DRAWING_PROMPT_V2, MangaPageDrawingPromptV2};
use a4n_error::AppError;
use serde_json::Value;

pub(crate) fn page_drawing_prompt_from_response(
    json: &Value,
    expected_page_number: u32,
    context: &str,
) -> Result<MangaPageDrawingPromptV2, AppError> {
    let parsed: MangaPageDrawingPromptV2 =
        serde_json::from_value(json.clone()).map_err(|error| {
            AppError::InvalidArgument(format!(
                "{context} 不符合 {MANGA_PAGE_DRAWING_PROMPT_V2} 合同: {error}"
            ))
        })?;
    parsed
        .validate(expected_page_number)
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    Ok(parsed)
}
