mod build;
pub(super) mod formatting;
pub(super) mod json;
pub(super) mod selection;

pub use json::{compact_json_text, is_meaningful_json_value, truncate_text_chars};
pub use selection::outline_character_names;
