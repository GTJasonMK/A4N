use super::super::normalize_image_metadata;
use a4n_core::models::ai::image_gen_config;
use serde_json::{Value, json};

pub(super) struct ImageGenerationTrace<'a> {
    pub(super) command: &'a str,
    pub(super) config: &'a image_gen_config::Model,
    pub(super) image_type: &'a str,
    pub(super) prompt: &'a str,
    pub(super) provider_model: &'a str,
    pub(super) chapter_id: Option<i64>,
    pub(super) panel_id: Option<&'a str>,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) generated_at: chrono::NaiveDateTime,
}

pub(super) fn attach_image_generation_trace(
    metadata_json: Option<Value>,
    trace: ImageGenerationTrace<'_>,
) -> Result<Option<Value>, String> {
    let generation = json!({
        "command": trace.command,
        "config_id": trace.config.id,
        "config_name": &trace.config.name,
        "backend": &trace.config.backend,
        "base_url": &trace.config.base_url,
        "configured_model": &trace.config.model,
        "provider_model": trace.provider_model,
        "image_type": trace.image_type,
        "chapter_id": trace.chapter_id,
        "panel_id": trace.panel_id,
        "width": trace.width,
        "height": trace.height,
        "prompt": trace.prompt,
        "generated_at": trace.generated_at.to_string(),
        "runtime_config_snapshot": crate::runtime_config::redacted_current_snapshot_value(),
    });
    let metadata = match metadata_json {
        Some(Value::Object(mut map)) => {
            map.insert("generation".into(), generation);
            Some(Value::Object(map))
        }
        Some(_) => return Err("图片生成 metadata_json 必须是对象".to_string()),
        None => Some(json!({
            "generation": generation,
        })),
    };
    Ok(normalize_image_metadata(metadata))
}

pub(super) fn image_generation_media_refs(metadata_json: &Value) -> Option<Value> {
    let references = metadata_json
        .get("generation_context")
        .and_then(|context| context.get("reference_image_ids"))
        .or_else(|| metadata_json.get("reference_image_ids"))
        .cloned();
    let bindings = metadata_json
        .get("reference_bindings")
        .cloned()
        .or_else(|| {
            metadata_json
                .get("generation_context")
                .and_then(|context| context.get("reference_bindings").cloned())
        });
    match (references, bindings) {
        (Some(references), Some(bindings)) => Some(json!({
            "reference_image_ids": references,
            "reference_bindings": bindings,
        })),
        (Some(references), None) => Some(json!({
            "reference_image_ids": references,
        })),
        (None, Some(bindings)) => Some(json!({
            "reference_bindings": bindings,
        })),
        (None, None) => None,
    }
}
