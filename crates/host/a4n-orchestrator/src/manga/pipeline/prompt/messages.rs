mod common;
mod inputs;
mod page_pipeline;
mod page_planning;
mod preprocessing;

#[cfg(test)]
pub(in crate::manga) use common::build_manga_step_user_message;
pub(in crate::manga) use inputs::manga_page_source_input;
#[cfg(test)]
pub(in crate::manga) use inputs::{manga_page_drawing_prompt_page_input, manga_page_outline_input};
pub(in crate::manga) use page_pipeline::{
    manga_page_adaptation_user_message, manga_page_drawing_prompt_pipeline_user_message,
    manga_page_reference_discovery_user_message, manga_page_storyboard_user_message,
};
pub(in crate::manga) use page_planning::{
    MangaPageContentMessages, manga_page_outline_user_message,
};
pub(in crate::manga) use preprocessing::{MangaPreprocessingMessages, manga_preprocessing_summary};
