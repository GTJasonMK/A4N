use crate::project::{
    import_chapter_outline_status, import_chapter_status, optional_json_f32_field,
    optional_json_object_clone, optional_json_positive_i32_field, optional_json_positive_i64_field,
    optional_json_string, optional_json_value, optional_top_array, optional_top_object,
    required_json_array, required_json_bool, required_json_nonnegative_i32_field,
    required_json_object, required_json_positive_i32_field, required_json_string,
    required_json_text,
};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

mod blueprint;
mod chapters;
mod contracts;
mod mapping;
mod outlines;

use blueprint::*;
use chapters::*;
use contracts::*;
use outlines::*;

/// 解析并校验文本项目完整导入 JSON 的 payload 段。
pub(crate) fn parse_import_payload(root: &Value) -> Result<ImportPayload, String> {
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
