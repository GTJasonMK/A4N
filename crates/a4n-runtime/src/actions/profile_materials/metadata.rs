use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ProfileMaterialStyleMetadata {
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

pub(crate) fn profile_material_metadata(
    kind: &str,
    profile_id: i64,
    profile_name: &str,
    profile_content_sha256: &str,
    style: &ProfileMaterialStyleMetadata,
) -> Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("material_kind".to_string(), json!("profile_material"));
    metadata.insert("profile_kind".to_string(), json!(kind));
    metadata.insert("profile_id".to_string(), json!(profile_id));
    metadata.insert("profile_name".to_string(), json!(profile_name));
    metadata.insert(
        "profile_content_sha256".to_string(),
        json!(profile_content_sha256),
    );
    for (key, value) in [
        ("style_template_id", style.style_template_id.as_deref()),
        ("style_template_name", style.style_template_name.as_deref()),
        (
            "style_template_version",
            style.style_template_version.as_deref(),
        ),
        ("custom_style", style.custom_style.as_deref()),
    ] {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            metadata.insert(key.to_string(), json!(value));
        }
    }
    Value::Object(metadata)
}
