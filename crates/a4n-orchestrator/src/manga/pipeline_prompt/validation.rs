mod common;
mod early;
mod late;
mod synthesis;

pub(in crate::manga) use early::{
    validate_manga_character_binding_source_span_ids,
    validate_manga_speech_selection_source_span_ids,
};
pub(in crate::manga) use late::{
    validate_manga_item_extraction_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
pub(in crate::manga) use synthesis::validate_manga_chapter_comic_synthesis;
