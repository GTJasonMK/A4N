use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use serde_json::{Value, json};

use a4n_domain::source_spans::SourceSpan;

use super::common::build_manga_step_user_message;
use super::inputs::manga_item_extraction_input;
use super::source_windows::event_source_span_windows;
use crate::source_markers::SourceMarkerMap;

pub(in crate::manga) fn manga_visual_anchor_extraction_user_message(
    events_json: Value,
    source_spans: &[SourceSpan],
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    let chapter_context = event_source_span_windows(&events_json, source_spans, source_markers)?;
    let input = source_markers.encode_input(&json!({
        "chapter": chapter_context,
        "previous_step": {"events": events_json}
    }))?;
    build_manga_step_user_message(
        "manga_visual_anchor_extraction",
        "基于事件窗口和事件卡片提取场景边界、地点氛围与视觉锚点",
        input,
        json!({
            "format": "json_object",
            "prompt_name": "manga_visual_anchor_extraction",
            "required_fields": ["scenes"],
            "source_span_rule": "scenes[] 每项必须输出 source_span_ids；只能选择 input.chapter.available_source_span_ids 中的 #序号；新增或无原文依据的历史节拍时为空数组，不得自造 ID"
        }),
    )
}

pub(in crate::manga) fn manga_item_extraction_user_message(
    speech_json: Value,
    visual_json: Value,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    build_manga_step_user_message(
        "manga_item_extraction",
        "基于对白旁白和视觉锚点抽取漫画页需要稳定绘制的关键物品",
        source_markers.encode_input(&manga_item_extraction_input(speech_json, visual_json)?)?,
        json!({
            "format": "json_object",
            "prompt_name": "manga_item_extraction",
            "required_fields": ["items"],
            "source_span_rule": "items[] 每项必须输出 source_span_ids；只能选择 input.available_source_span_ids 中的 #序号；新增或无原文依据的历史节拍使用空数组；没有关键物品时 items 输出空数组"
        }),
    )
}

pub(in crate::manga) fn manga_chapter_comic_synthesis_user_message(
    events_json: Value,
    speech_json: Value,
    visual_json: Value,
    items_json: Value,
    source_markers: &SourceMarkerMap,
) -> Result<LlmContextManifest, AppError> {
    build_manga_step_user_message(
        "manga_chapter_comic_synthesis",
        "基于事件、对白旁白、视觉锚点和物品生成漫画分页所需章节综合信息",
        source_markers.encode_input(&json!({
            "previous_steps": {
                "events": events_json,
                "speech": speech_json,
                "visual_anchors": visual_json,
                "items": items_json
            }
        }))?,
        json!({
            "format": "json_object",
            "prompt_name": "manga_chapter_comic_synthesis",
            "required_fields": ["chapter_summary", "mood_progression", "total_estimated_pages"],
            "limits": {
                "total_estimated_pages_min": 5,
                "total_estimated_pages_max": 15
            }
        }),
    )
}
