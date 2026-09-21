use std::collections::HashSet;

use a4n_domain::manga::MangaPageV3;
use a4n_error::AppError;
use serde_json::{Map, Value, json};

use super::super::validation::required_manga_string;

pub(in crate::manga::pipeline_prompt) fn character_names_for_page(
    page_plan: &MangaPageV3,
) -> HashSet<String> {
    page_plan
        .panels
        .iter()
        .flat_map(|panel| panel.participants.iter().cloned())
        .collect()
}

pub(in crate::manga::pipeline_prompt) fn filter_characters(
    value: &Value,
    names: &HashSet<String>,
) -> Result<Value, AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("漫画绘图上下文 characters 必须是对象".to_string())
    })?;
    Ok(Value::Object(
        object
            .iter()
            .filter(|(key, _)| names.contains(key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Map<_, _>>(),
    ))
}

pub(in crate::manga::pipeline_prompt) fn filter_reference_assets(
    value: &Value,
    names: &HashSet<String>,
) -> Result<Value, AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::InvalidArgument("漫画绘图上下文 reference_assets 必须是对象".to_string())
    })?;
    let source_portraits = object
        .get("character_portraits")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument(
                "漫画绘图上下文 reference_assets.character_portraits 必须是数组".to_string(),
            )
        })?;
    let mut portraits = Vec::new();
    for (index, portrait) in source_portraits.iter().enumerate() {
        let path = format!("reference_assets.character_portraits[{index}]");
        let name = required_manga_string(portrait, "character_name", &path)?;
        required_manga_string(portrait, "prompt", &path)?;
        if names.contains(&name) {
            portraits.push(portrait.clone());
        }
    }

    Ok(json!({
        "character_portraits": portraits,
    }))
}
