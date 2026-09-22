mod contracts;
mod fingerprint;
mod llm;
mod messages;
mod page_context;
mod validation;

pub(super) use contracts::{MangaLlmCallInput, MangaPromptGenerationTrace};
pub(super) use fingerprint::manga_pipeline_prompt_fingerprint;
pub(super) use llm::{
    MangaRenderTraceInput, load_manga_restored_llm_checkpoint, prepare_manga_llm_call,
};
#[cfg(test)]
pub(super) use messages::{
    build_manga_step_user_message, manga_item_extraction_input,
    manga_page_drawing_prompt_page_input, manga_page_planning_input,
};
pub(super) use messages::{
    manga_chapter_comic_synthesis_user_message, manga_character_binding_user_message,
    manga_item_available_source_span_ids, manga_item_extraction_user_message,
    manga_page_drawing_prompt_user_message, manga_page_planning_user_message,
    manga_speech_selection_user_message, manga_visual_anchor_extraction_user_message,
};
pub(super) use validation::{
    validate_manga_chapter_comic_synthesis, validate_manga_character_binding_source_span_ids,
    validate_manga_item_extraction_source_span_ids,
    validate_manga_speech_selection_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
