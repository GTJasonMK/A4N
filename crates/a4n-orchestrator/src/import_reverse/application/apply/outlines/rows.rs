use a4n_db::import_reverse::{ReverseOutlineApplyRow, ReversePartOutlineApplyRow};
use serde_json::Value;

use super::super::validation::{
    optional_positive_i32, required_array_clone, required_object_clone, required_positive_i32,
    validate_reverse_outline_items, validate_reverse_part_outline_items,
};
use crate::import_reverse::application::reverse_json::{
    reject_unknown_keys, required_array, required_string,
};

pub fn reverse_outline_rows(data: &Value) -> Result<Vec<ReverseOutlineApplyRow>, String> {
    reject_unknown_keys(data, &["outlines"], "reverse_outline")?;
    let outlines = required_array(data, "outlines", "reverse_outline")?;
    validate_reverse_outline_items(outlines)?;
    let mut rows = Vec::with_capacity(outlines.len());

    for (index, item) in outlines.iter().enumerate() {
        let context = format!("reverse_outline.outlines[{index}]");
        let chapter_number = required_positive_i32(item, "chapter_number", &context)?;
        let title = required_string(item, "title", &context)?;
        let summary = required_string(item, "summary", &context)?;
        let word_count_target = optional_positive_i32(item, "word_count_target", &context)?;
        rows.push(ReverseOutlineApplyRow {
            chapter_number,
            title,
            summary,
            word_count_target,
        });
    }

    Ok(rows)
}

pub fn reverse_part_outline_rows(data: &Value) -> Result<Vec<ReversePartOutlineApplyRow>, String> {
    reject_unknown_keys(data, &["parts"], "reverse_part_outline")?;
    let parts = required_array(data, "parts", "reverse_part_outline")?;
    validate_reverse_part_outline_items(parts)?;
    let mut rows = Vec::with_capacity(parts.len());

    for (index, item) in parts.iter().enumerate() {
        let context = format!("reverse_part_outline.parts[{index}]");
        let part_number = required_positive_i32(item, "part_number", &context)?;
        let title = required_string(item, "title", &context)?;
        let start_chapter = required_positive_i32(item, "start_chapter", &context)?;
        let end_chapter = required_positive_i32(item, "end_chapter", &context)?;
        let summary = required_string(item, "summary", &context)?;
        let theme = required_string(item, "theme", &context)?;
        let key_events = required_array_clone(item, "key_events", &context)?;
        let character_focus = required_object_clone(item, "character_focus", &context)?;
        rows.push(ReversePartOutlineApplyRow {
            part_number,
            title,
            start_chapter,
            end_chapter,
            summary,
            theme,
            key_events,
            character_focus,
        });
    }

    Ok(rows)
}
