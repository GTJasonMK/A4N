// Extraction is intentionally kept separate from chapter entry formatting.
mod characters;

#[cfg(test)]
mod tests;

pub(super) use characters::{collect_extracted_characters, dedup_character_records};

mod foreshadowing;
pub(super) use foreshadowing::collect_analysis_foreshadowing;
