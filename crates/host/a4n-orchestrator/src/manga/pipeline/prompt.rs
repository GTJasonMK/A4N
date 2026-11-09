mod contracts;
mod fingerprint;
mod llm;
mod messages;
mod page_context;
mod validation;

pub(in crate::manga) use contracts::{
    MangaLlmCallInput, MangaPromptGenerationTrace, manga_pipeline_prompt_assembly_call_key,
};
pub(in crate::manga) use fingerprint::manga_pipeline_prompt_fingerprint;
pub(in crate::manga) use llm::{
    MangaRenderTraceInput, load_manga_restored_llm_checkpoint, prepare_manga_llm_call,
};
pub(in crate::manga) use messages::{
    MangaPageContentMessages, MangaPreprocessingMessages, manga_page_adaptation_user_message,
    manga_page_drawing_prompt_pipeline_user_message, manga_page_outline_user_message,
    manga_page_reference_discovery_user_message, manga_page_source_input,
    manga_page_storyboard_user_message, manga_preprocessing_summary,
};
#[cfg(test)]
pub(in crate::manga) use messages::{
    build_manga_step_user_message, manga_page_drawing_prompt_page_input, manga_page_outline_input,
};
