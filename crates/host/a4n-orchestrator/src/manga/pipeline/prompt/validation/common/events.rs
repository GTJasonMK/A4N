use std::collections::HashSet;

use crate::manga::adaptation_projection::MangaBeatIndex;
use a4n_domain::media_authoring::AdaptationBeatRef;
use a4n_error::AppError;
use serde_json::Value;

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_nonnegative_index(
    value: Option<&Value>,
    path: &str,
) -> Result<i64, AppError> {
    let index = value
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是非负整数")))?;
    if index < 0 {
        return Err(AppError::InvalidArgument(format!("{path} 必须是非负整数")));
    }
    Ok(index)
}

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_beat_ref(
    value: Option<&Value>,
    path: &str,
    known_beats: &MangaBeatIndex,
) -> Result<AdaptationBeatRef, AppError> {
    let value = value.ok_or_else(|| AppError::InvalidArgument(format!("{path} 缺失")))?;
    let beat_ref: AdaptationBeatRef = serde_json::from_value(value.clone())
        .map_err(|error| AppError::InvalidArgument(format!("{path} 无效: {error}")))?;
    beat_ref
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("{path}.{error}")))?;
    if !known_beats.contains_key(&beat_ref) {
        return Err(AppError::InvalidArgument(format!(
            "{path} 引用了不存在的改编节拍: unit_id={}, beat_id={}",
            beat_ref.unit_id, beat_ref.beat_id
        )));
    }
    Ok(beat_ref)
}

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_beat_refs(
    value: Option<&Value>,
    path: &str,
    known_beats: &MangaBeatIndex,
) -> Result<Vec<AdaptationBeatRef>, AppError> {
    let refs = value
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    if refs.is_empty() {
        return Err(AppError::InvalidArgument(format!("{path} 不能为空")));
    }

    let mut seen = HashSet::with_capacity(refs.len());
    for (index, value) in refs.iter().enumerate() {
        let beat_ref =
            validate_manga_beat_ref(Some(value), &format!("{path}[{index}]"), known_beats)?;
        if !seen.insert(beat_ref.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "{path} 包含重复改编节拍引用：unit_id={}, beat_id={}",
                beat_ref.unit_id, beat_ref.beat_id
            )));
        }
    }
    Ok(seen.into_iter().collect())
}

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_adaptation_sources(
    ids: &[String],
    references: &[AdaptationBeatRef],
    known_beats: &MangaBeatIndex,
    source_spans: &super::source::SourceSpanIndex<'_>,
    path: &str,
) -> Result<(), AppError> {
    use a4n_domain::media_authoring::{AdaptationOrigin, validate_adaptation_evidence};
    let mut origins = Vec::new();
    let mut available = std::collections::BTreeSet::new();
    for reference in references {
        let beat = known_beats
            .get(reference)
            .ok_or_else(|| AppError::InvalidArgument(format!("{path} 引用了未知改编节拍")))?;
        origins.push(beat.origin);
        available.extend(beat.source_span_ids.iter().cloned());
    }
    let origin = if origins
        .iter()
        .any(|origin| matches!(origin, AdaptationOrigin::Source | AdaptationOrigin::Adapted))
    {
        AdaptationOrigin::Adapted
    } else if origins
        .iter()
        .all(|origin| *origin == AdaptationOrigin::Added)
    {
        AdaptationOrigin::Added
    } else {
        AdaptationOrigin::Historical
    };
    validate_adaptation_evidence(origin, ids, &available)
        .map_err(|error| AppError::InvalidArgument(format!("{path}: {error}")))?;
    for id in ids {
        source_spans
            .resolve_text(id)
            .map_err(|error| AppError::InvalidArgument(format!("{path}: {error}")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn typed_beat_reference_must_exist_before_the_next_stage() {
        let known = crate::manga::tests::known_beats(&[], 1);
        let original = json!({"unit_id": "unit-1", "beat_id": "beat-1"});
        let mut value = original.clone();
        assert!(validate_manga_beat_ref(Some(&value), "beat_ref", &known).is_ok());
        for field in ["unit_id", "beat_id"] {
            value[field] = json!("other");
            let error = validate_manga_beat_ref(Some(&value), "beat_ref", &known).unwrap_err();
            assert!(error.to_string().contains("不存在的改编节拍"));
            value[field] = original[field].clone();
        }
    }
}
