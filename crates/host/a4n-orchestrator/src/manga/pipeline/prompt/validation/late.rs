use std::collections::HashSet;

use a4n_error::AppError;
use serde_json::Value;

use a4n_domain::source_spans::SourceSpan;

use super::common::{
    SourceSpanIndex, reject_manga_unknown_fields, required_manga_enum, required_manga_string,
    validate_manga_beat_ref, validate_manga_beat_refs, validate_manga_nonnegative_index,
    validate_manga_nullable_string, validate_manga_source_span_id_array,
};

pub(in crate::manga) fn validate_manga_visual_anchor_source_span_ids(
    visual_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(visual_json, &["scenes"], "manga_visual_anchor_extraction")?;
    let scenes = visual_json
        .get("scenes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("manga_visual_anchor_extraction.scenes 必须是数组".into())
        })?;
    let source_spans = SourceSpanIndex::new(source_spans);
    for (index, scene) in scenes.iter().enumerate() {
        let path = format!("manga_visual_anchor_extraction.scenes[{index}]");
        reject_manga_unknown_fields(
            scene,
            &[
                "index",
                "location",
                "location_en",
                "time_of_day",
                "atmosphere",
                "weather",
                "lighting",
                "indoor_outdoor",
                "description",
                "beat_refs",
                "source_span_ids",
            ],
            &path,
        )?;
        validate_manga_nonnegative_index(scene.get("index"), &format!("{path}.index"))?;
        required_manga_string(scene, "location", &path)?;
        required_manga_string(scene, "location_en", &path)?;
        required_manga_string(scene, "time_of_day", &path)?;
        required_manga_string(scene, "atmosphere", &path)?;
        validate_manga_nullable_string(scene.get("weather"), &format!("{path}.weather"))?;
        required_manga_string(scene, "lighting", &path)?;
        required_manga_enum(scene, "indoor_outdoor", &path, &["indoor", "outdoor"])?;
        required_manga_string(scene, "description", &path)?;
        validate_manga_source_span_id_array(
            scene,
            &source_spans,
            &format!("{path}.source_span_ids"),
        )?;
        validate_manga_beat_refs(scene.get("beat_refs"), &format!("{path}.beat_refs"))?;
    }
    Ok(())
}

pub(in crate::manga) fn validate_manga_item_extraction_source_span_ids(
    items_json: &Value,
    source_spans: &[SourceSpan],
    allowed_source_span_ids: &[String],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(items_json, &["items"], "manga_item_extraction")?;
    let items = items_json
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("manga_item_extraction.items 必须是数组".into())
        })?;
    let source_spans = SourceSpanIndex::new(source_spans);
    let allowed = allowed_source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    for (index, item) in items.iter().enumerate() {
        let path = format!("manga_item_extraction.items[{index}]");
        reject_manga_unknown_fields(
            item,
            &[
                "name",
                "description",
                "description_en",
                "importance",
                "first_appearance_beat_ref",
                "visual_features",
                "source_span_ids",
            ],
            &path,
        )?;
        required_manga_string(item, "name", &path)?;
        required_manga_string(item, "description", &path)?;
        required_manga_string(item, "description_en", &path)?;
        required_manga_string(item, "importance", &path)?;
        required_manga_string(item, "visual_features", &path)?;
        let source_span_path = format!("{path}.source_span_ids");
        validate_manga_source_span_id_array(item, &source_spans, &source_span_path)?;
        validate_manga_beat_ref(
            item.get("first_appearance_beat_ref"),
            &format!("{path}.first_appearance_beat_ref"),
        )?;
        let ids = item
            .get("source_span_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::InvalidArgument(format!("{source_span_path} 必须是数组")))?;
        for (id_index, id) in ids.iter().enumerate() {
            let source_span_id = id
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "{source_span_path}[{id_index}] 必须是非空字符串"
                    ))
                })?;
            if !allowed.contains(source_span_id) {
                return Err(AppError::InvalidArgument(format!(
                    "{source_span_path}[{id_index}] 不在 input.available_source_span_ids 中: {source_span_id}"
                )));
            }
        }
    }
    Ok(())
}
