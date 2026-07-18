use super::super::contracts::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_PLAN_V3, MangaPageDrawingPromptV2, MangaPagePlanV3,
};
use super::MangaPlanValidationError;
use super::page::validate_page;
use super::rules::{index_u32, invalid, required_text};

pub(super) fn validate_plan(plan: &MangaPagePlanV3) -> Result<(), MangaPlanValidationError> {
    if plan.schema_version != MANGA_PAGE_PLAN_V3 {
        return invalid("schema_version", format!("必须等于 {MANGA_PAGE_PLAN_V3}"));
    }
    if plan.shooting_script_id <= 0 {
        return invalid("shooting_script_id", "必须大于 0");
    }
    if plan.shooting_script_revision <= 0 {
        return invalid("shooting_script_revision", "必须大于 0");
    }
    if plan.pages.is_empty() {
        return invalid("pages", "至少需要一页");
    }
    let mut previous_exit_state: Option<&str> = None;
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
        if let Some(previous_exit_state) = previous_exit_state
            && page.entry_state.trim() != previous_exit_state.trim()
        {
            return invalid(
                format!("{path}.entry_state"),
                "必须与上一页 exit_state 一致，避免页面间状态跳变",
            );
        }
        previous_exit_state = Some(&page.exit_state);
    }
    Ok(())
}

pub(super) fn validate_drawing_prompt(
    prompt: &MangaPageDrawingPromptV2,
    expected_page_number: u32,
) -> Result<(), MangaPlanValidationError> {
    if prompt.schema_version != MANGA_PAGE_DRAWING_PROMPT_V2 {
        return invalid(
            "schema_version",
            format!("必须等于 {MANGA_PAGE_DRAWING_PROMPT_V2}"),
        );
    }
    if prompt.page_number != expected_page_number {
        return invalid("page_number", format!("必须等于 {expected_page_number}"));
    }
    required_text(&prompt.image_prompt, "image_prompt")
}
