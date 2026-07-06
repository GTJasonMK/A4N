mod blueprint;
mod character;
mod json_format;
mod outline;
mod scene;

pub(super) use blueprint::{
    blueprint_metadata_rag_record, blueprint_rag_record, blueprint_world_setting_rag_records,
    relationship_rag_record,
};
#[cfg(test)]
pub(super) use blueprint::{
    blueprint_metadata_rag_text, blueprint_rag_text, blueprint_world_setting_sections,
    expected_blueprint_world_setting_record_count, relationship_rag_text,
};
pub(super) use character::{blueprint_character_rag_records, protagonist_rag_records};
#[cfg(test)]
pub(super) use character::{
    blueprint_character_rag_sections, blueprint_character_rag_text, character_state_rag_text,
    expected_blueprint_character_record_count, expected_protagonist_record_count,
    foreshadowing_rag_text, protagonist_rag_sections, protagonist_rag_text,
};
pub(crate) use character::{
    character_attribute_event_rag_record, character_state_rag_record, foreshadowing_rag_record,
};
pub(super) use outline::{outline_rag_record, part_outline_rag_record};
#[cfg(test)]
pub(super) use outline::{outline_rag_text, part_outline_rag_text};
pub(super) use scene::{
    scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};

use serde_json::Value;

#[derive(Debug, Clone)]
pub(super) struct DimensionRagSection {
    pub(super) dimension: &'static str,
    pub(super) label: &'static str,
    pub(super) text: String,
}

pub(super) fn required_rag_text(
    value: Option<&str>,
    context: impl Into<String>,
) -> Result<&str, String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| context.into())
}

pub(super) fn optional_rag_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|text| !text.is_empty())
}

pub(super) fn optional_json_text(value: Option<&Value>) -> Option<String> {
    value
        .map(json_format::format_json_for_rag)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

pub(super) fn short_rag_title(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let mut title = trimmed.chars().take(max_chars).collect::<String>();
    title.push_str("...");
    title
}

pub(super) fn dimension_fields_section(
    category: &'static str,
    dimension: &'static str,
    label: &'static str,
    name: &str,
    fields: &[(&str, Option<&str>)],
    include_identity_only: bool,
) -> Option<DimensionRagSection> {
    let field_lines = fields
        .iter()
        .filter_map(|(field_label, value)| {
            optional_rag_text(*value).map(|value| format!("{field_label}: {value}"))
        })
        .collect::<Vec<_>>();
    if field_lines.is_empty() && !include_identity_only {
        return None;
    }

    let mut lines = vec![format!("[{category}-{label}]"), format!("角色: {name}")];
    lines.extend(field_lines);
    Some(DimensionRagSection {
        dimension,
        label,
        text: lines.join("\n"),
    })
}

pub(super) fn dimension_text_section(
    category: &'static str,
    dimension: &'static str,
    label: &'static str,
    name: &str,
    field_label: &str,
    text: Option<String>,
) -> Option<DimensionRagSection> {
    let text = text
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())?;
    Some(DimensionRagSection {
        dimension,
        label,
        text: format!("[{category}-{label}]\n角色: {name}\n{field_label}: {text}"),
    })
}

#[cfg(test)]
pub(super) fn format_character_record(name: &str, fields: &[(&str, Option<&str>)]) -> String {
    let mut lines = vec![format!("角色: {name}")];
    for (label, value) in fields {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            lines.push(format!("{label}: {value}"));
        }
    }
    lines.join("\n")
}
