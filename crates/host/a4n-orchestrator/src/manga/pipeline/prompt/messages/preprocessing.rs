use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use serde_json::json;

use super::common::build_manga_step_user_message;
use crate::source_markers::SourceMarkerMap;

pub(in crate::manga) fn manga_preprocessing_user_message(
    chapter_content: &str,
    source_spans: &[SourceSpan],
    source_markers: &SourceMarkerMap,
    reference_assets: serde_json::Value,
) -> Result<LlmContextManifest, AppError> {
    let chapter = source_markers.mark_text(chapter_content, source_spans)?;
    build_manga_step_user_message(
        "manga_preprocessing",
        "从原文中整理漫画改编前的章节事实和章节级分页节奏参考，输出一个可供改编计划与分页共同使用的预处理包。",
        source_markers.encode_input(&json!({
            "chapter": {"content": chapter},
            "reference_assets": reference_assets,
        }))?,
        json!({
            "format": "json_object",
            "prompt_name": "manga_preprocessing",
            "schema_version": "manga_preprocessing_v3",
            "required_fields": ["schema_version", "source_events", "characters", "speech", "visual_anchors", "items", "comic_synthesis"],
            "source_event_rule": "source_events.events[] 必须有唯一 event_id、description、participants、source_span_ids；只能引用输入正文短标签",
            "speech_rule": "speech.dialogues/narrations 使用 source_event_id，不使用 beat_ref",
            "visual_rule": "visual_anchors.scenes[] 使用 source_event_ids，不使用 beat_refs",
            "item_rule": "items.items[] 使用 first_appearance_source_event_id，不使用 first_appearance_beat_ref",
            "no_page_level_rule": "不得输出 unit_id、beat_id、页数、改编取舍或具体分页内容；comic_synthesis 只提供章节叙事重点和情绪递进"
        }),
    )
}
