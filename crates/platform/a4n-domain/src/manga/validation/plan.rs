use super::super::contracts::{MangaPageDrawingPromptV2, MangaPagePlanV4};
use super::MangaPlanValidationError;
use super::page::validate_page;
use super::rules::{index_u32, invalid, required_text};

pub(super) fn validate_plan(plan: &MangaPagePlanV4) -> Result<(), MangaPlanValidationError> {
    if plan.adaptation_plan_id <= 0 {
        return invalid("adaptation_plan_id", "必须大于 0");
    }
    if plan.adaptation_plan_revision <= 0 {
        return invalid("adaptation_plan_revision", "必须大于 0");
    }
    if plan.pages.is_empty() {
        return invalid("pages", "至少需要一页");
    }
    for (page_offset, page) in plan.pages.iter().enumerate() {
        let expected = index_u32(page_offset, "pages")?;
        let path = format!("pages[{page_offset}]");
        if page.page_number != expected {
            return invalid(
                format!("{path}.page_number"),
                format!("必须从 1 连续递增，期望 {expected}"),
            );
        }
        validate_page(page, &path)?;
    }
    Ok(())
}

pub(super) fn validate_drawing_prompt(
    prompt: &MangaPageDrawingPromptV2,
    expected_page_number: u32,
) -> Result<(), MangaPlanValidationError> {
    if prompt.page_number != expected_page_number {
        return invalid("page_number", format!("必须等于 {expected_page_number}"));
    }
    required_text(&prompt.image_prompt, "image_prompt")
}
