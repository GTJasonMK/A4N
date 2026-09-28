use crate::manga::adaptation_projection::MangaBeatIndex;
use a4n_error::AppError;
use serde_json::Value;

use a4n_domain::source_spans::SourceSpan;

use super::common::{
    SourceSpanIndex, reject_manga_unknown_fields, required_manga_bool, required_manga_string,
    validate_manga_adaptation_sources, validate_manga_beat_ref,
    validate_manga_characters_object_shape, validate_manga_nonnegative_index,
    validate_manga_nullable_string,
};

pub(in crate::manga) fn validate_manga_character_binding_source_span_ids(
    characters_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(characters_json, &["characters"], "manga_character_binding")?;
    let characters = characters_json
        .get("characters")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AppError::InvalidArgument("manga_character_binding.characters 必须是对象".into())
        })?;
    let source_spans = SourceSpanIndex::new(source_spans);
    validate_manga_characters_object_shape(
        characters,
        &source_spans,
        "manga_character_binding.characters",
    )
}

pub(in crate::manga) fn validate_manga_speech_selection_source_span_ids(
    speech_json: &Value,
    source_spans: &[SourceSpan],
    known_beats: &MangaBeatIndex,
) -> Result<(), AppError> {
    reject_manga_unknown_fields(
        speech_json,
        &["dialogues", "narrations"],
        "manga_speech_selection",
    )?;
    let source_spans = SourceSpanIndex::new(source_spans);
    validate_speech_array(speech_json, &source_spans, "dialogues", known_beats)?;
    validate_speech_array(speech_json, &source_spans, "narrations", known_beats)?;
    Ok(())
}

fn validate_speech_array(
    speech_json: &Value,
    source_spans: &SourceSpanIndex<'_>,
    field: &str,
    known_beats: &MangaBeatIndex,
) -> Result<(), AppError> {
    let context = "manga_speech_selection";
    let items = speech_json
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{field} 必须是数组")))?;
    for (index, item) in items.iter().enumerate() {
        let item_context = format!("{context}.{field}[{index}]");
        let (allowed, is_dialogue) = match field {
            "dialogues" => (
                &[
                    "index",
                    "speaker",
                    "content",
                    "source_span_id",
                    "emotion",
                    "target",
                    "beat_ref",
                    "is_internal",
                    "bubble_type",
                ][..],
                true,
            ),
            "narrations" => (
                &[
                    "index",
                    "content",
                    "source_span_id",
                    "narration_type",
                    "beat_ref",
                    "position",
                ][..],
                false,
            ),
            other => {
                return Err(AppError::InvalidArgument(format!(
                    "manga_speech_selection 不支持字段 {other}"
                )));
            }
        };
        reject_manga_unknown_fields(item, allowed, &item_context)?;
        validate_manga_nonnegative_index(item.get("index"), &format!("{item_context}.index"))?;
        required_manga_string(item, "content", &item_context)?;
        if is_dialogue {
            required_manga_string(item, "speaker", &item_context)?;
            required_manga_string(item, "emotion", &item_context)?;
            validate_manga_nullable_string(item.get("target"), &format!("{item_context}.target"))?;
            required_manga_bool(
                item.get("is_internal"),
                &format!("{item_context}.is_internal"),
            )?;
            required_manga_string(item, "bubble_type", &item_context)?;
        } else {
            required_manga_string(item, "narration_type", &item_context)?;
            required_manga_string(item, "position", &item_context)?;
        }
        let ids = match item.get("source_span_id") {
            Some(Value::Null) => Vec::new(),
            Some(Value::String(id)) if !id.trim().is_empty() => vec![id.clone()],
            _ => {
                return Err(AppError::InvalidArgument(format!(
                    "{item_context}.source_span_id 必须明确提供引用或 null"
                )));
            }
        };
        let reference = validate_manga_beat_ref(
            item.get("beat_ref"),
            &format!("{context}.{field}[{index}].beat_ref"),
            known_beats,
        )?;
        validate_manga_adaptation_sources(
            &ids,
            &[reference],
            known_beats,
            source_spans,
            &format!("{item_context}.source_span_id"),
        )?;
    }
    Ok(())
}
