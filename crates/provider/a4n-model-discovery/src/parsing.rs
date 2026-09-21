use std::collections::HashSet;

use serde_json::Value;

use crate::endpoints::ModelDiscoveryParser;
use a4n_model_discovery_contract::DiscoveredModel;

pub(crate) fn parse_discovered_models(
    value: &Value,
    parser: ModelDiscoveryParser,
) -> Result<Vec<DiscoveredModel>, String> {
    let models = match parser {
        ModelDiscoveryParser::DataList => parse_data_list_models(value)?,
        ModelDiscoveryParser::Gemini => parse_gemini_models(value)?,
        ModelDiscoveryParser::StableDiffusion => parse_stable_diffusion_models(value)?,
        ModelDiscoveryParser::ComfyUiCheckpointLoader => parse_comfyui_checkpoint_models(value)?,
    };
    {
        let mut seen = HashSet::new();
        for model in &models {
            if !seen.insert(model.id.as_str()) {
                return Err(format!("模型 ID 重复：{}", model.id));
            }
        }
    }
    Ok(models)
}

fn parse_data_list_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "缺少 data[]".to_string())?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            model_from_data_list_item(item).map_err(|error| format!("data[{index}] {error}"))
        })
        .collect()
}

fn parse_gemini_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .get("models")
        .and_then(Value::as_array)
        .ok_or_else(|| "缺少 models[]".to_string())?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            model_from_gemini_item(item).map_err(|error| format!("models[{index}] {error}"))
        })
        .collect()
}

fn parse_stable_diffusion_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| "SD-WebUI sd-models 响应必须是数组".to_string())?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let id = required_trimmed_json_string(
                item.get("model_name"),
                &format!("sd-models[{index}].model_name"),
            )?;
            Ok(DiscoveredModel {
                id: id.to_string(),
                name: trimmed_json_string(item.get("title")).map(str::to_string),
                owned_by: None,
            })
        })
        .collect()
}

fn parse_comfyui_checkpoint_models(value: &Value) -> Result<Vec<DiscoveredModel>, String> {
    let ckpt_name = value
        .pointer("/CheckpointLoaderSimple/input/required/ckpt_name/0")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "ComfyUI 响应缺少 CheckpointLoaderSimple.input.required.ckpt_name".to_string()
        })?;
    ckpt_name
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let id = required_trimmed_json_string(
                Some(item),
                &format!("CheckpointLoaderSimple ckpt_name[{index}]"),
            )?;
            Ok(DiscoveredModel {
                id: id.to_string(),
                name: None,
                owned_by: None,
            })
        })
        .collect()
}

fn model_from_data_list_item(value: &Value) -> Result<DiscoveredModel, String> {
    let item = value.as_object().ok_or_else(|| "必须是对象".to_string())?;
    let id = required_trimmed_json_string(item.get("id"), "id")?;
    Ok(DiscoveredModel {
        id: id.to_string(),
        name: trimmed_json_string(item.get("name"))
            .or_else(|| trimmed_json_string(item.get("display_name")))
            .map(str::to_string),
        owned_by: trimmed_json_string(item.get("owned_by")).map(str::to_string),
    })
}

fn model_from_gemini_item(value: &Value) -> Result<DiscoveredModel, String> {
    let item = value.as_object().ok_or_else(|| "必须是对象".to_string())?;
    let raw_name = required_trimmed_json_string(item.get("name"), "name")?;
    let id = strip_models_prefix(raw_name).to_string();
    Ok(DiscoveredModel {
        id,
        name: trimmed_json_string(item.get("displayName")).map(str::to_string),
        owned_by: None,
    })
}

fn trimmed_json_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).and_then(non_empty_trimmed)
}

fn required_trimmed_json_string<'a>(
    value: Option<&'a Value>,
    field: &str,
) -> Result<&'a str, String> {
    trimmed_json_string(value).ok_or_else(|| format!("{field} 必须是非空字符串"))
}

fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn strip_models_prefix(value: &str) -> &str {
    value.strip_prefix("models/").unwrap_or(value)
}
