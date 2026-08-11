use super::pipeline_prompt::{
    build_manga_step_user_message, manga_item_available_source_span_ids,
    manga_item_extraction_input, manga_page_drawing_prompt_page_input, manga_page_planning_input,
    manga_speech_selection_user_message, manga_visual_anchor_extraction_user_message,
    validate_manga_character_binding_source_span_ids,
    validate_manga_item_extraction_source_span_ids,
    validate_manga_speech_selection_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
use a4n_core::source_spans::source_spans_for_content;
use serde_json::json;

mod context;
mod messages;
mod validation;
