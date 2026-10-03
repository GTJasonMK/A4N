use std::collections::HashSet;

use a4n_domain::{
    manga::{
        MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_SLOTS_V1, MangaPageAdaptationV1,
        MangaPageContentV1, MangaPageV4,
    },
    source_spans::SourceSpan,
};
use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use serde_json::{Value, json};

use super::common::build_manga_step_user_message;
use super::inputs::{manga_page_drawing_prompt_page_input, manga_page_slots_input};
use crate::source_markers::SourceMarkerMap;

pub(in crate::manga) fn manga_page_slots_user_message(
    extraction_summary: Value,
    adaptation_plan: Value,
    source_spans: &[SourceSpan],
    chapter_content: &str,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    let mut input = manga_page_slots_input(extraction_summary, adaptation_plan)?;
    input
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("分页内容上下文必须是 JSON 对象".into()))?
        .insert(
            "source".into(),
            json!(source_markers.mark_text(chapter_content, source_spans)?),
        );
    let input = source_markers.encode_input(&input)?;
    build_manga_step_user_message(
        "manga_page_slots",
        "只确定页数和每页承载的剧情内容，不规划画格或图像提示词",
        input,
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_slots",
            "schema_version": MANGA_PAGE_SLOTS_V1,
            "required_fields": ["schema_version", "pages"],
            "page_count_rule": "由 pages 数组的长度确定页数；每页只输出一条 content，不输出 panel 或绘图描述",
            "source_span_rule": "source_span_ids 只能引用 input.source 中出现的 #序号标记",
            "beat_ref_rule": "beat_refs 只能引用 input.adaptation_plan.units[].beats 中的节拍；所有 required 节拍必须覆盖",
        }),
    )
}

pub(in crate::manga) fn manga_page_adaptation_user_message(
    page_number: u32,
    total_pages: usize,
    content: &MangaPageContentV1,
    adaptation_plan: Value,
    previous_content: Option<&MangaPageContentV1>,
    next_content: Option<&MangaPageContentV1>,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    let input = source_markers.encode_input(&json!({
        "page_number": page_number,
        "total_pages": total_pages,
        "page_content": content,
        "adaptation_beats": adaptation_plan,
        "previous_page_content": previous_content,
        "next_page_content": next_content,
    }))?;
    build_manga_step_user_message(
        "manga_page_adaptation_plan",
        &format!("为第 {page_number}/{total_pages} 页制定独立的页面改编计划"),
        input,
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_adaptation_plan",
            "required_fields": ["visual_intent", "continuity_requirements", "retained_details", "omitted_details", "beat_refs"],
            "beat_ref_rule": "beat_refs 必须且只能包含 page_content.beat_refs 中的节拍",
        }),
    )
}

pub(in crate::manga) fn manga_page_storyboard_user_message(
    page_number: u32,
    total_pages: usize,
    content: &MangaPageContentV1,
    adaptation: &MangaPageAdaptationV1,
    previous_content: Option<&MangaPageContentV1>,
    next_content: Option<&MangaPageContentV1>,
    adaptation_plan: Value,
    extraction_summary: &Value,
    reference_assets: &Value,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    let participants = plan_participants(&adaptation_plan, reference_assets);
    let input = source_markers.encode_input(&json!({
        "page_number": page_number,
        "total_pages": total_pages,
        "page_content": content,
        "page_adaptation": adaptation,
        "adaptation_beats": adaptation_plan,
        "neighboring_pages": {
            "previous_content": previous_content,
            "next_content": next_content,
        },
        "characters": extraction_summary.get("characters"),
        "reference_assets": participants,
    }))?;
    build_manga_step_user_message(
        "manga_page_storyboard",
        &format!("只为第 {page_number}/{total_pages} 页编排画格和文字层"),
        input,
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_storyboard",
            "required_fields": ["page_number", "beat_refs", "role", "pacing", "importance", "reading_flow", "page_goal", "visual_thesis", "entry_state", "exit_state", "dominant_panel_number", "panels"],
            "page_number": page_number,
            "beat_ref_rule": "beat_refs 必须覆盖 page_adaptation.beat_refs；每格 beat_refs 必须属于页面 beat_refs",
        }),
    )
}

pub(in crate::manga) fn manga_page_drawing_prompt_pipeline_user_message(
    page_number: u32,
    page: &MangaPageV4,
    content: &MangaPageContentV1,
    adaptation: &MangaPageAdaptationV1,
    extraction_summary: &Value,
    reference_assets: &Value,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    let mut input =
        manga_page_drawing_prompt_page_input(page, extraction_summary, reference_assets)?;
    let input = input
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("绘图提示词上下文必须是 JSON 对象".into()))?;
    input.insert("page_content".into(), serde_json::to_value(content)?);
    input.insert("page_adaptation".into(), serde_json::to_value(adaptation)?);
    let input = source_markers.encode_input(&Value::Object(input.clone()))?;
    build_manga_step_user_message(
        "manga_page_drawing_prompt",
        &format!("只为第 {page_number} 页生成一个 {MANGA_PAGE_DRAWING_PROMPT_V2} 严格 JSON 对象"),
        input,
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_drawing_prompt",
            "mode": "single_page",
            "schema_version": MANGA_PAGE_DRAWING_PROMPT_V2,
            "page_number": page_number,
        }),
    )
}

fn plan_participants(adaptation_plan: &Value, reference_assets: &Value) -> Value {
    let participants = adaptation_plan
        .get("units")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|unit| unit.get("participants").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<HashSet<_>>();
    let portraits = reference_assets
        .get("character_portraits")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|portrait| {
            portrait
                .get("character_name")
                .and_then(Value::as_str)
                .is_some_and(|name| participants.contains(name))
        })
        .cloned()
        .collect::<Vec<_>>();
    json!({"character_portraits": portraits})
}
