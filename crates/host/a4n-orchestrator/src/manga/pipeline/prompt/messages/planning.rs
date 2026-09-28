use a4n_domain::manga::{MANGA_PAGE_DRAWING_PROMPT_V2, MangaPageV4};
use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use serde_json::{Value, json};

use super::common::build_manga_step_user_message;
use super::inputs::{manga_page_drawing_prompt_page_input, manga_page_planning_input};

pub(in crate::manga) fn manga_page_planning_user_message(
    extraction_summary: Value,
    adaptation_plan: Value,
) -> Result<LlmContextManifest, AppError> {
    build_manga_step_user_message(
        "manga_page_planning",
        "基于完整抽取信息规划漫画分页结构",
        manga_page_planning_input(extraction_summary, adaptation_plan)?,
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_planning"
        }),
    )
}

pub(in crate::manga) fn manga_page_drawing_prompt_user_message(
    page_number: u32,
    page_plan: &MangaPageV4,
    extraction_summary: &Value,
    reference_assets: &Value,
) -> Result<LlmContextManifest, AppError> {
    build_manga_step_user_message(
        "manga_page_drawing_prompt",
        &format!("只为第 {page_number} 页生成一个 {MANGA_PAGE_DRAWING_PROMPT_V2} 严格 JSON 对象。"),
        manga_page_drawing_prompt_page_input(page_plan, extraction_summary, reference_assets)?,
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_drawing_prompt",
            "mode": "single_page",
            "schema_version": MANGA_PAGE_DRAWING_PROMPT_V2,
            "page_number": page_number
        }),
    )
}
