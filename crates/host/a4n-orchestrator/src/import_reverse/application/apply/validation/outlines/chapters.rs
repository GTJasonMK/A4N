use std::collections::BTreeSet;

use serde_json::Value;

use super::super::fields::{optional_positive_i32, required_positive_i32};
use crate::import_reverse::application::reverse_json::{reject_unknown_keys, required_string};

pub(in crate::import_reverse::application::apply) fn validate_reverse_outline_items(
    outlines: &[Value],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for (index, item) in outlines.iter().enumerate() {
        let context = format!("reverse_outline.outlines[{index}]");
        reject_unknown_keys(
            item,
            &["chapter_number", "title", "summary", "word_count_target"],
            &context,
        )?;
        let chapter_number = required_positive_i32(item, "chapter_number", &context)?;
        if !seen.insert(chapter_number) {
            return Err(format!(
                "reverse_outline.outlines 包含重复章节: {chapter_number}"
            ));
        }
        required_string(item, "title", &context)?;
        required_string(item, "summary", &context)?;
        optional_positive_i32(item, "word_count_target", &context)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
