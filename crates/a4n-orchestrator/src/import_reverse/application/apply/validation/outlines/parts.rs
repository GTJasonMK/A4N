use std::collections::BTreeSet;

use serde_json::Value;

use super::super::fields::{required_array_clone, required_object_clone, required_positive_i32};
use super::ranges::validate_part_chapter_ranges;
use crate::import_reverse::application::reverse_json::{reject_unknown_keys, required_string};

pub(in crate::import_reverse::application::apply) fn validate_reverse_part_outline_items(
    parts: &[Value],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut ranges = Vec::with_capacity(parts.len());
    for (index, item) in parts.iter().enumerate() {
        let context = format!("reverse_part_outline.parts[{index}]");
        reject_unknown_keys(
            item,
            &[
                "part_number",
                "title",
                "start_chapter",
                "end_chapter",
                "summary",
                "theme",
                "key_events",
                "character_focus",
            ],
            &context,
        )?;
        let part_number = required_positive_i32(item, "part_number", &context)?;
        if !seen.insert(part_number) {
            return Err(format!(
                "reverse_part_outline.parts 包含重复分部: {part_number}"
            ));
        }
        let start_chapter = required_positive_i32(item, "start_chapter", &context)?;
        let end_chapter = required_positive_i32(item, "end_chapter", &context)?;
        if end_chapter < start_chapter {
            return Err(format!("{context}.end_chapter 不能小于 start_chapter"));
        }
        ranges.push((start_chapter, end_chapter, part_number));
        required_string(item, "title", &context)?;
        required_string(item, "summary", &context)?;
        required_string(item, "theme", &context)?;
        required_array_clone(item, "key_events", &context)?;
        required_object_clone(item, "character_focus", &context)?;
    }
    validate_part_chapter_ranges(&mut ranges)?;
    Ok(())
}

#[cfg(test)]
mod tests;
