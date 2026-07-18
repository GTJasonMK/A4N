use serde_json::{Value, json};

use super::VideoAssetCandidate;

pub(super) fn video_library_image_entry(
    candidate: &VideoAssetCandidate,
    display_order: usize,
) -> Value {
    let image = &candidate.image;
    let metadata = image.metadata_json.as_ref();
    let role = non_empty_text(Some(image.image_type.as_str()));
    let label = metadata_nested_text(metadata, "generation_context", "title")
        .or_else(|| metadata_text(metadata, "profile_name"))
        .or_else(|| metadata_text(metadata, "character_name"))
        .or_else(|| non_empty_text(Some(image.image_type.as_str())));
    let prompt_excerpt = non_empty_text(Some(image.prompt.as_str())).map(truncate_asset_text);
    let description = metadata_nested_text(metadata, "generation_context", "scene_summary")
        .or_else(|| metadata_nested_text(metadata, "generation_context", "prompt_summary"))
        .or_else(|| metadata_text(metadata, "description"))
        .map(str::to_string)
        .or_else(|| prompt_excerpt.clone());
    json!({
        "image_id": image.id,
        "display_order": display_order,
        "display_label": format!("参考图 {display_order}"),
        "project_id": image.project_id,
        "role": role,
        "label": label,
        "description": description,
        "prompt_excerpt": prompt_excerpt,
        "selection_reason": candidate.selection_reason,
        "profile_kind": candidate.profile_kind,
        "profile_id": candidate.profile_id,
        "is_primary": candidate.is_primary,
        "is_favorite": image.is_favorite,
    })
}

fn truncate_asset_text(text: &str) -> String {
    const MAX_CHARS: usize = 180;
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    let excerpt = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{excerpt}...")
    } else {
        excerpt
    }
}

fn metadata_text<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        .and_then(|text| non_empty_text(Some(text)))
}

fn metadata_nested_text<'a>(
    metadata: Option<&'a Value>,
    object_key: &str,
    text_key: &str,
) -> Option<&'a str> {
    metadata
        .and_then(|metadata| metadata.get(object_key))
        .and_then(|object| object.get(text_key))
        .and_then(Value::as_str)
        .and_then(|text| non_empty_text(Some(text)))
}

fn non_empty_text(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}
