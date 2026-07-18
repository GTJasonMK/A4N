use super::super::super::ProfileMaterialStyleMetadata;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

pub(super) fn profile_source_material(parts: &[&str], prompt: &str) -> String {
    let mut source = parts.join("\n");
    source.push('\n');
    source.push_str(prompt);
    source
}

pub(super) fn profile_material_style_metadata(
    style_template_id: Option<String>,
    style_template_name: Option<String>,
    style_template_version: Option<String>,
    custom_style: Option<String>,
) -> ProfileMaterialStyleMetadata {
    ProfileMaterialStyleMetadata {
        style_template_id,
        style_template_name,
        style_template_version,
        custom_style,
    }
}

pub(super) fn profile_image_metadata(
    kind: &str,
    profile_id: i64,
    profile_name: &str,
    profile_content: &str,
    source_material: &str,
    prompt: &str,
    style_metadata: &ProfileMaterialStyleMetadata,
) -> Value {
    let mut metadata = super::super::super::profile_material_metadata(
        kind,
        profile_id,
        profile_name,
        &source_content_sha256(profile_content),
        style_metadata,
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert("source".to_string(), json!(format!("{kind}_profile")));
        map.insert("chapter_video_material".to_string(), json!(true));
        map.insert("prompt_snapshot".to_string(), json!(prompt));
        map.insert(
            "source_content_sha256".to_string(),
            json!(source_content_sha256(source_material)),
        );
    }
    metadata
}
