use serde_json::{Map, Value};

use super::super::analysis_schema_error;
use super::super::fields::{
    reject_unknown_fields, required_array, required_object, required_string,
};

pub(super) fn validate_foreshadowing_schema(
    root: &Map<String, Value>,
) -> Result<(), a4n_error::AppError> {
    let foreshadowing = required_object(root, "foreshadowing", "chapter_analysis")?;
    reject_unknown_fields(
        foreshadowing,
        &["planted", "resolved", "tensions", "progressed"],
        "chapter_analysis.foreshadowing",
    )?;
    let planted = required_array(foreshadowing, "planted", "chapter_analysis.foreshadowing")?;
    for (index, item) in planted.iter().enumerate() {
        let context = format!("chapter_analysis.foreshadowing.planted[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| analysis_schema_error(format!("{context} 必须是对象")))?;
        reject_unknown_fields(
            item,
            &[
                "id",
                "description",
                "original_text",
                "category",
                "priority",
                "related_entities",
            ],
            &context,
        )?;
        required_string(item, "description", &context)?;
        required_string(item, "original_text", &context)?;
        required_string(item, "category", &context)?;
        required_string(item, "priority", &context)?;
        let related_entities = required_array(item, "related_entities", &context)?;
        for (related_index, related) in related_entities.iter().enumerate() {
            if related.as_str().map(str::trim).is_none_or(str::is_empty) {
                return Err(analysis_schema_error(format!(
                    "{context}.related_entities[{related_index}] 必须是非空字符串"
                )));
            }
        }
    }
    let resolved = required_array(foreshadowing, "resolved", "chapter_analysis.foreshadowing")?;
    for (index, item) in resolved.iter().enumerate() {
        let context = format!("chapter_analysis.foreshadowing.resolved[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| analysis_schema_error(format!("{context} 必须是对象")))?;
        reject_unknown_fields(item, &["id", "resolution", "source_span_ids"], &context)?;
        required_string(item, "id", &context)?;
        required_string(item, "resolution", &context)?;
    }
    let tensions = required_array(foreshadowing, "tensions", "chapter_analysis.foreshadowing")?;
    for (index, item) in tensions.iter().enumerate() {
        if item.as_str().map(str::trim).is_none_or(str::is_empty) {
            return Err(analysis_schema_error(format!(
                "chapter_analysis.foreshadowing.tensions[{index}] 必须是非空字符串"
            )));
        }
    }
    if let Some(progressed) = foreshadowing.get("progressed") {
        let progressed = progressed
            .as_array()
            .ok_or_else(|| analysis_schema_error("foreshadowing.progressed 必须是数组"))?;
        for item in progressed {
            let item = item
                .as_object()
                .ok_or_else(|| analysis_schema_error("伏笔进展必须是对象"))?;
            reject_unknown_fields(
                item,
                &["id", "status", "resolution", "source_span_ids"],
                "foreshadowing.progressed",
            )?;
            required_string(item, "id", "foreshadowing.progressed")?;
            required_string(item, "resolution", "foreshadowing.progressed")?;
            let status = required_string(item, "status", "foreshadowing.progressed")?;
            if !matches!(status, "partially_revealed" | "abandoned") {
                return Err(analysis_schema_error(
                    "伏笔进展状态只能是 partially_revealed 或 abandoned",
                ));
            }
            let ids: Vec<String> =
                serde_json::from_value(item.get("source_span_ids").cloned().unwrap_or(Value::Null))
                    .map_err(|error| analysis_schema_error(format!("伏笔进展证据无效: {error}")))?;
            a4n_domain::fact_feedback::validate_evidence_ids(&ids)
                .map_err(analysis_schema_error)?;
        }
    }
    Ok(())
}
