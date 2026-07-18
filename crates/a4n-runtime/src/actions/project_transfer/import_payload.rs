use super::validation::{
    optional_json_positive_i32_field, optional_json_string, required_json_array,
    required_json_nonnegative_i32_field, required_json_object, required_json_positive_i32_field,
    required_json_string, required_json_text, required_text,
};
use a4n_core::models::chapter::{chapter, chapter_outline};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod blueprint;
mod chapters;
mod contracts;
mod json_rules;
mod mapping;
mod outlines;

use blueprint::*;
use chapters::*;
use contracts::*;
use json_rules::*;
use outlines::*;

pub(super) fn parse_import_payload(root: &Value) -> Result<ImportPayload, String> {
    let chapters = parse_import_chapters(root)?;
    let part_outlines = parse_import_part_outlines(root)?;
    let chapter_outlines = parse_import_chapter_outlines(root)?;
    validate_import_chapter_outline_part_references(&chapter_outlines, &part_outlines)?;
    validate_import_chapter_outline_references(&chapters, &chapter_outlines)?;
    Ok(ImportPayload {
        chapters,
        blueprint: parse_import_blueprint(root)?,
        blueprint_versions: parse_import_blueprint_versions(root)?,
        characters: parse_import_characters(root)?,
        relationships: parse_import_relationships(root)?,
        part_outlines,
        chapter_outlines,
    })
}

#[cfg(test)]
mod tests;
