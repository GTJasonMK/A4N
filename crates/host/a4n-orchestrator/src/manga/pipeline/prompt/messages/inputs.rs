use std::collections::HashSet;

use a4n_domain::manga::MangaPageV4;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::page_context::{
    character_names_for_page, filter_characters, filter_reference_assets,
};
use super::source_windows::push_unique_source_span_id;

pub(in crate::manga) fn manga_item_extraction_input(
    speech: Value,
    visual_anchors: Value,
) -> Result<Value, AppError> {
    let available_source_span_ids = manga_item_available_source_span_ids(&speech, &visual_anchors)?;
    Ok(json!({
        "available_source_span_ids": available_source_span_ids,
        "previous_steps": {
            "speech": speech,
            "visual_anchors": visual_anchors
        }
    }))
}

pub(in crate::manga) fn manga_item_available_source_span_ids(
    speech: &Value,
    visual_anchors: &Value,
) -> Result<Vec<String>, AppError> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for field in ["dialogues", "narrations"] {
        let items = speech.get(field).and_then(Value::as_array).ok_or_else(|| {
            AppError::InvalidArgument(format!("manga_speech_selection.{field} 必须是数组"))
        })?;
        for (index, item) in items.iter().enumerate() {
            if item.get("source_span_id").is_some_and(Value::is_null) {
                continue;
            }
            push_unique_source_span_id(
                item.get("source_span_id"),
                &format!("manga_speech_selection.{field}[{index}].source_span_id"),
                &mut ids,
                &mut seen,
            )?;
        }
    }
    let scenes = visual_anchors
        .get("scenes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument(
                "manga_visual_anchor_extraction.scenes 必须是数组".to_string(),
            )
        })?;
    for (scene_index, scene) in scenes.iter().enumerate() {
        let source_span_ids = scene
            .get("source_span_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "manga_visual_anchor_extraction.scenes[{scene_index}].source_span_ids 必须是数组"
                ))
            })?;
        for (id_index, id) in source_span_ids.iter().enumerate() {
            push_unique_source_span_id(
                Some(id),
                &format!(
                    "manga_visual_anchor_extraction.scenes[{scene_index}].source_span_ids[{id_index}]"
                ),
                &mut ids,
                &mut seen,
            )?;
        }
    }
    Ok(ids)
}

pub(in crate::manga) fn manga_page_slots_input(
    extraction_summary: Value,
    adaptation_plan: Value,
) -> Result<Value, AppError> {
    let required = |key: &str| {
        extraction_summary
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument(format!("漫画分页规划上下文缺少 {key}")))
    };
    Ok(json!({
        "extraction_summary": {
            "characters": required("characters")?,
            "speech": required("speech")?,
            "visual_anchors": required("visual_anchors")?,
            "items": required("items")?,
            "comic_synthesis": required("comic_synthesis")?,
        },
        "adaptation_plan": adaptation_plan
    }))
}

pub(in crate::manga) fn manga_page_drawing_prompt_page_input(
    page_plan: &MangaPageV4,
    extraction_summary: &Value,
    reference_assets: &Value,
) -> Result<Value, AppError> {
    let character_names = character_names_for_page(page_plan);
    let characters = filter_characters(
        extraction_summary.get("characters").ok_or_else(|| {
            AppError::InvalidArgument("漫画绘图上下文缺少 characters".to_string())
        })?,
        &character_names,
    )?;
    let reference_assets = filter_reference_assets(reference_assets, &character_names)?;
    Ok(json!({
        "current_page": page_plan,
        "page_context": {
            "characters": characters,
        },
        "reference_assets": reference_assets,
    }))
}
