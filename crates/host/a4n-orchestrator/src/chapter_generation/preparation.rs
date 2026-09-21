mod outline;
mod reference_context;
mod variables;
mod word_count;

pub use outline::{generation_chapter_title, merge_generation_outline_text};
pub use reference_context::{
    compact_json_text, is_meaningful_json_value, outline_character_names, truncate_text_chars,
};
pub use variables::{non_empty_trimmed, upsert_chapter_generation_variables};
pub use word_count::{ChapterGenerationWordCountRange, resolve_generation_word_count_range};

#[cfg(test)]
mod tests;
