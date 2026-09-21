use super::rules::optional_rag_text;

#[derive(Debug, Clone)]
pub struct DimensionRagSection {
    pub dimension: &'static str,
    pub label: &'static str,
    pub text: String,
}

pub fn dimension_fields_section(
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

pub fn dimension_text_section(
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
