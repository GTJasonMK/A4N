mod contracts;
mod fingerprint;
mod llm;
mod messages;
mod page_context;
mod validation;

pub(in crate::manga) use contracts::{MangaLlmCallInput, MangaPromptGenerationTrace};
pub(in crate::manga) use fingerprint::manga_pipeline_prompt_fingerprint;
pub(in crate::manga) use llm::{
    MangaRenderTraceInput, load_manga_restored_llm_checkpoint, prepare_manga_llm_call,
};
#[cfg(test)]
pub(in crate::manga) use messages::{
    build_manga_step_user_message, manga_page_drawing_prompt_page_input, manga_page_slots_input,
};
pub(in crate::manga) use messages::{
    manga_page_adaptation_user_message, manga_page_drawing_prompt_pipeline_user_message,
    manga_page_reference_discovery_user_message, manga_page_slots_user_message,
    manga_page_storyboard_user_message, manga_preprocessing_user_message,
};
pub(in crate::manga) use validation::validate_manga_preprocessing;
