use a4n_core::models::asset::object_profile;
use a4n_core::models::scene::scene_profile;
use serde_json::Value;

use super::ChapterVideoSourceContext;

pub(super) fn is_chapter_video_material(metadata: Option<&Value>) -> bool {
    let Some(metadata) = metadata.and_then(Value::as_object) else {
        return false;
    };
    if super::super::is_video_frame_input_metadata(metadata) {
        return false;
    }
    metadata
        .get("chapter_video_material")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(super) fn source_relevance_text(source: &ChapterVideoSourceContext<'_>) -> String {
    let mut parts = vec![source.content.to_string()];
    if let Some(summary) = source.summary {
        parts.push(summary.to_string());
    }
    if let Some(analysis) = source.analysis {
        parts.push(analysis.to_string());
    }
    parts.join("\n")
}

pub(super) fn scene_profile_relevance(
    scene: &scene_profile::Model,
    chapter_number: i32,
    source_text: &str,
) -> Option<usize> {
    profile_relevance(
        &scene.name,
        &scene.aliases_json,
        scene.first_seen_chapter,
        scene.last_seen_chapter,
        chapter_number,
        source_text,
    )
}

pub(super) fn object_profile_relevance(
    object: &object_profile::Model,
    chapter_number: i32,
    source_text: &str,
) -> Option<usize> {
    profile_relevance(
        &object.name,
        &object.aliases_json,
        object.first_seen_chapter,
        object.last_seen_chapter,
        chapter_number,
        source_text,
    )
}

pub(super) fn profile_relevance(
    name: &str,
    aliases: &Value,
    first_seen_chapter: i32,
    last_seen_chapter: i32,
    chapter_number: i32,
    source_text: &str,
) -> Option<usize> {
    if profile_name_matches_source(name, aliases, source_text) {
        return Some(0);
    }
    (chapter_number >= first_seen_chapter && chapter_number <= last_seen_chapter).then_some(1)
}

fn profile_name_matches_source(name: &str, aliases: &Value, source_text: &str) -> bool {
    let source_text = source_text.trim();
    if source_text.is_empty() {
        return false;
    }
    if text_contains_non_empty(source_text, name) {
        return true;
    }
    aliases
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|alias| text_contains_non_empty(source_text, alias))
}

fn text_contains_non_empty(haystack: &str, needle: &str) -> bool {
    let needle = needle.trim();
    !needle.is_empty() && haystack.contains(needle)
}
