pub fn portrait_style_with_pose(style: Option<&str>, pose: Option<&str>) -> Option<String> {
    let style = style.map(str::trim).filter(|value| !value.is_empty());
    let pose = pose.map(str::trim).filter(|value| !value.is_empty());
    match (style, pose) {
        (Some(style), Some(pose)) => Some(format!("{style}\n姿势要求：{pose}")),
        (Some(style), None) => Some(style.to_string()),
        (None, Some(pose)) => Some(format!("姿势要求：{pose}")),
        (None, None) => None,
    }
}

pub fn profile_portrait_seed_prompt(
    profile: &protagonist_profile::Model,
) -> Result<String, String> {
    let mut parts = vec![
        character_sheet_seed_instruction(),
        format!("角色名：{}", profile.name),
    ];
    for (label, value) in [
        ("性别", profile.gender.as_deref()),
        ("年龄", profile.age.as_deref()),
        ("外观", profile.appearance.as_deref()),
        ("性格", profile.personality.as_deref()),
        ("背景", profile.background.as_deref()),
    ] {
        if let Some(text) = optional_profile_text(value) {
            parts.push(format!("{label}：{text}"));
        }
    }
    let profile_context = portrait_profile_context(profile);
    if matches!(profile_context.as_object(), Some(fields) if !fields.is_empty()) {
        let context = serde_json::to_string(&profile_context).map_err(|err| err.to_string())?;
        parts.push(format!("角色档案上下文：{context}"));
    }
    Ok(parts.join("，"))
}

pub fn optional_profile_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn first_expression_or_default(expressions: Option<&[String]>) -> String {
    expressions
        .unwrap_or_default()
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PORTRAIT_EXPRESSION)
        .to_string()
}

pub fn portrait_profile_context(profile: &protagonist_profile::Model) -> Value {
    let mut object = Map::new();
    insert_meaningful_json(&mut object, "base_attributes", &profile.base_attributes);
    insert_meaningful_json(&mut object, "current_state", &profile.current_state);
    if let Some(arc) = profile.character_arc.as_ref() {
        insert_meaningful_json(&mut object, "character_arc", arc);
    }
    if let Some(voice) = profile.voice_fingerprint.as_ref() {
        insert_meaningful_json(&mut object, "voice_fingerprint", voice);
    }
    Value::Object(object)
}

fn insert_meaningful_json(object: &mut Map<String, Value>, key: &str, value: &Value) {
    if is_meaningful_json(value) {
        object.insert(key.to_string(), value.clone());
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}
use super::super::character_sheet_seed_instruction;
use a4n_core::models::character::protagonist_profile;
use serde_json::{Map, Value};

pub const DEFAULT_PORTRAIT_EXPRESSION: &str = "neutral";
