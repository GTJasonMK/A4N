use std::collections::HashSet;

use serde_json::{Value, json};

use super::super::page_context::{
    character_names_for_page, filter_characters, filter_reference_assets,
};
use super::source_windows::push_unique_source_span_id;

pub(in crate::manga) fn manga_item_extraction_input(speech: Value, visual_anchors: Value) -> Value {
    let available_source_span_ids = manga_item_available_source_span_ids(&speech, &visual_anchors);
    json!({
        "available_source_span_ids": available_source_span_ids,
        "previous_steps": {
            "speech": speech,
            "visual_anchors": visual_anchors
        }
    })
}

pub(in crate::manga) fn manga_item_available_source_span_ids(
    speech: &Value,
    visual_anchors: &Value,
) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for field in ["dialogues", "narrations"] {
        for item in speech
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            push_unique_source_span_id(item.get("source_span_id"), &mut ids, &mut seen);
        }
    }
    for scene in visual_anchors
        .get("scenes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for id in scene
            .get("source_span_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            push_unique_source_span_id(Some(id), &mut ids, &mut seen);
        }
    }
    ids
}

pub(in crate::manga) fn manga_page_planning_input(
    extraction_summary: Value,
    shooting_script: Value,
) -> Value {
    json!({
        "extraction_summary": {
            "characters": extraction_summary["characters"].clone(),
            "speech": extraction_summary["speech"].clone(),
            "visual_anchors": extraction_summary["visual_anchors"].clone(),
            "items": extraction_summary["items"].clone(),
            "comic_synthesis": extraction_summary["comic_synthesis"].clone(),
        },
        "shooting_script": shooting_script
    })
}

pub(in crate::manga) fn manga_page_drawing_prompt_page_input(
    page_plan: &Value,
    extraction_summary: &Value,
) -> Value {
    let character_names = character_names_for_page(page_plan, &[]);
    let characters = filter_characters(extraction_summary.get("characters"), &character_names);
    let reference_assets =
        filter_reference_assets(extraction_summary.get("reference_assets"), &character_names);
    json!({
        "current_page": page_plan,
        "page_context": {
            "characters": characters,
        },
        "reference_assets": reference_assets,
    })
}
