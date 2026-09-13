use a4n_error::AppError;
use a4n_llm::context::LlmContextManifest;
use serde_json::{Value, json};

use a4n_domain::source_spans::SourceSpan;
use a4n_domain::source_spans::source_span_prompt_values;

use super::common::build_manga_step_user_message;
use super::source_windows::event_source_span_windows;

pub(in crate::manga) fn manga_character_binding_user_message(
    chapter_content: &str,
    events_json: Value,
    source_spans: &[SourceSpan],
    reference_assets: Value,
) -> Result<LlmContextManifest, AppError> {
    build_manga_step_user_message(
        "manga_character_binding",
        "基于事件卡片、章节正文和角色立绘参考绑定本章出场角色身份与外观",
        json!({
            "chapter": {
                "content": chapter_content,
                "source_spans": source_span_prompt_values(source_spans)
            },
            "previous_step": {
                "events": events_json
            },
            "reference_assets": reference_assets
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_character_binding",
            "required_fields": ["characters"],
            "source_span_rule": "characters.*.source_span_ids 必须从 input.chapter.source_spans 中选择，至少 1 个，不要自造 ID"
        }),
    )
}

pub(in crate::manga) fn manga_speech_selection_user_message(
    characters_json: Value,
    events_json: Value,
    source_spans: &[SourceSpan],
    reference_assets: Value,
) -> Result<LlmContextManifest, AppError> {
    let chapter_context = event_source_span_windows(&events_json, source_spans)?;
    build_manga_step_user_message(
        "manga_speech_selection",
        "基于事件窗口、角色绑定和事件卡片，选择关键对白、想法与旁白",
        json!({
            "chapter": chapter_context,
            "previous_steps": {
                "characters": characters_json,
                "events": events_json
            },
            "reference_assets": reference_assets
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_speech_selection",
            "required_fields": ["dialogues", "narrations"],
            "source_span_rule": "dialogues[] 和 narrations[] 每项必须输出 source_span_id；只能从 input.chapter.available_source_spans 中选择，不要自造 ID",
            "limits": {
                "dialogues_max": 24,
                "narrations_max": 8,
                "dialogue_content_max_chars": 30,
                "narration_content_max_chars": 40
            }
        }),
    )
}
