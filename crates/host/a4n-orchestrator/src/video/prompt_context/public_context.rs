use serde_json::Value;

use crate::video::prompt_json::VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS;

const FINAL_VIDEO_PROMPT_INTERNAL_CONTEXT_KEYS: &[&str] = &[
    "source_span_id",
    "source_span_ids",
    "key_source_span_ids",
    "source_span_range",
    "reference_image_ids",
    "reference_binding",
    "image_id",
    "asset_id",
    "source_id",
    "candidate_label",
    "source_display_label",
    "display_label",
    "display_order",
    "project_id",
    "profile_id",
];

pub(in crate::video) fn final_video_prompt_public_context(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(final_video_prompt_public_context)
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| {
                    !FINAL_VIDEO_PROMPT_INTERNAL_CONTEXT_KEYS.contains(&key.as_str())
                })
                .map(|(key, value)| {
                    let value = if key == "dialogue_beats" {
                        final_video_prompt_public_dialogue_cues(value)
                    } else {
                        final_video_prompt_public_context(value)
                    };
                    let key = if key == "dialogue_beats" {
                        "dialogue_cues".to_string()
                    } else {
                        key.clone()
                    };
                    (key, value)
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn final_video_prompt_public_dialogue_cues(value: &Value) -> Value {
    let Some(items) = value.as_array() else {
        return final_video_prompt_public_context(value);
    };
    Value::Array(
        items
            .iter()
            .map(|item| {
                let Some(object) = item.as_object() else {
                    return final_video_prompt_public_context(item);
                };
                Value::Object(
                    object
                        .iter()
                        .filter(|(key, _)| {
                            VIDEO_PROMPT_DIALOGUE_BEAT_FIELDS.contains(&key.as_str())
                        })
                        .map(|(key, value)| (key.clone(), final_video_prompt_public_context(value)))
                        .collect(),
                )
            })
            .collect(),
    )
}
