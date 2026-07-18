mod objects;
mod response;
mod scope;

use super::types::ParsedObjectProfileDraft;
use a4n_core::models::{chapter::chapter, project::text_anchor};
use serde_json::Value;
use std::collections::BTreeSet;

#[cfg(test)]
pub(super) fn parse_and_validate_object_discovery_content(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<ParsedObjectProfileDraft>, String> {
    let (drafts, _) = parse_and_validate_object_discovery_output(
        content,
        analyzed_chapters,
        chapters,
        text_anchors,
    )?;
    Ok(drafts)
}

pub(super) fn parse_and_validate_object_discovery_output(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<(Vec<ParsedObjectProfileDraft>, Value), String> {
    let value = response::object_discovery_response_json(content)?;
    let drafts = response::parse_object_discovery_output_value(&value, chapters, text_anchors)?;
    scope::validate_object_chapter_scope(&drafts, analyzed_chapters)?;
    Ok((drafts, value))
}
