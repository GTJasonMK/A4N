use std::collections::HashSet;

use serde_json::Value;

use crate::contracts::DiscoveredModel;
use crate::endpoints::ModelDiscoveryParser;

pub(crate) fn parse_discovered_models(
    value: &Value,
    parser: ModelDiscoveryParser,
) -> Result<Vec<DiscoveredModel>, String> {
    let models = match parser {
        ModelDiscoveryParser::OpenAi => parse_openai_models(value)?,
        ModelDiscoveryParser::Gemini => parse_gemini_models(value)?,
        ModelDiscoveryParser::StableDiffusion => parse_stable_diffusion_models(value)?,
        ModelDiscoveryParser::ComfyUiCheckpointLoader => parse_comfyui_checkpoint_models(value)?,
    };
    Ok(dedupe_discovered_models(models))
}

fn parse_openai_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    if let Some(items) = value.get("data").and_then(Value::as_array) {
        return Ok(items.iter().filter_map(model_from_openai_item).collect());
    }
    if let Some(items) = value.get("models").and_then(Value::as_array) {
        return Ok(items.iter().filter_map(model_from_openai_item).collect());
    }
    if let Some(items) = value.as_array() {
        return Ok(items.iter().filter_map(model_from_openai_item).collect());
    }
    Err("缺少 data[] 或 models[]".to_string())
}

fn parse_gemini_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| "缺少 models[]".to_string())?;
    Ok(items.iter().filter_map(model_from_gemini_item).collect())
}

fn parse_stable_diffusion_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "SD-WebUI sd-models 响应必须是数组".to_string())?;
    Ok(items
        .iter()
        .filter_map(|item| {
            let id = trimmed_json_string(item.get("model_name"))
                .or_else(|| trimmed_json_string(item.get("title")))?;
            Some(DiscoveredModel {
                id: id.to_string(),
                name: trimmed_json_string(item.get("title")).map(str::to_string),
                owned_by: None,
            })
        })
        .collect())
}

fn parse_comfyui_checkpoint_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let ckpt_name = value
        .pointer("/CheckpointLoaderSimple/input/required/ckpt_name/0")
        .or_else(|| value.pointer("/input/required/ckpt_name/0"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "ComfyUI 响应缺少 CheckpointLoaderSimple.input.required.ckpt_name".to_string()
        })?;
    Ok(ckpt_name
        .iter()
        .filter_map(|item| {
            trimmed_json_string(Some(item)).map(|id| DiscoveredModel {
                id: id.to_string(),
                name: None,
                owned_by: None,
            })
        })
        .collect())
}

fn model_from_openai_item(value: &Value) -> Option<DiscoveredModel> {
    match value {
        Value::String(id) => non_empty_trimmed(id).map(|id| DiscoveredModel {
            id: id.to_string(),
            name: None,
            owned_by: None,
        }),
        Value::Object(item) => {
            let id = trimmed_json_string(item.get("id"))
                .or_else(|| trimmed_json_string(item.get("name")))
                .or_else(|| trimmed_json_string(item.get("model")))?;
            Some(DiscoveredModel {
                id: strip_models_prefix(id).to_string(),
                name: trimmed_json_string(item.get("name"))
                    .or_else(|| trimmed_json_string(item.get("display_name")))
                    .map(str::to_string),
                owned_by: trimmed_json_string(item.get("owned_by"))
                    .or_else(|| trimmed_json_string(item.get("owner")))
                    .map(str::to_string),
            })
        }
        _ => None,
    }
}

fn model_from_gemini_item(value: &Value) -> Option<DiscoveredModel> {
    let item = value.as_object()?;
    let raw_name = trimmed_json_string(item.get("name"))?;
    let id = strip_models_prefix(raw_name).to_string();
    Some(DiscoveredModel {
        id,
        name: trimmed_json_string(item.get("displayName"))
            .or_else(|| trimmed_json_string(item.get("display_name")))
            .map(str::to_string),
        owned_by: Some("google".to_string()),
    })
}

fn trimmed_json_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).and_then(non_empty_trimmed)
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn strip_models_prefix(value: &str) -> &str {
    value.trim().strip_prefix("models/").unwrap_or(value.trim())
}

fn dedupe_discovered_models(models: Vec<DiscoveredModel>) -> Vec<DiscoveredModel> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for model in models {
        if seen.insert(model.id.clone()) {
            deduped.push(model);
        }
    }
    deduped
}
