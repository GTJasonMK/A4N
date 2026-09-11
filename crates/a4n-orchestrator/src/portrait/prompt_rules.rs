//! 立绘提示词纯规则（无 DB、无 Host）。

use a4n_db_models::models::character::protagonist_profile;
use serde_json::{Map, Value};

use super::design_brief::character_sheet_seed_instruction;

/// 默认表情（未指定时）。
pub const DEFAULT_PORTRAIT_EXPRESSION: &str = "neutral";

/// 将风格与姿势合并为提示词 style 字段。
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

/// 由角色档案生成立绘种子提示词。
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

/// 可选档案文本：trim 后非空。
pub fn optional_profile_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// 规范化单一表情；未指定时使用业务默认值 neutral。
pub fn expression_or_default(expression: Option<&str>) -> String {
    expression
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PORTRAIT_EXPRESSION)
        .to_string()
}

/// 从角色档案提取有意义的 JSON 上下文（供提示词变量）。
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile_with(name: &str) -> protagonist_profile::Model {
        let ts = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: name.into(),
            gender: Some("女".into()),
            age: None,
            base_attributes: json!({"combat": "sword"}),
            current_state: json!({}),
            character_arc: None,
            voice_fingerprint: None,
            appearance: Some("黑发".into()),
            background: None,
            personality: None,
            last_synced_chapter: 0,
            created_at: ts,
            updated_at: ts,
        }
    }

    #[test]
    fn expression_defaults_to_neutral_and_trims_explicit_value() {
        assert_eq!(expression_or_default(None), "neutral");
        assert_eq!(expression_or_default(Some("  ")), "neutral");
        assert_eq!(expression_or_default(Some("  smile ")), "smile");
    }

    #[test]
    fn style_with_pose_merges() {
        assert_eq!(
            portrait_style_with_pose(Some("anime"), Some("站立")).as_deref(),
            Some("anime\n姿势要求：站立")
        );
        assert_eq!(
            portrait_style_with_pose(None, Some("坐")).as_deref(),
            Some("姿势要求：坐")
        );
    }

    #[test]
    fn seed_prompt_includes_name_and_context() {
        let prompt = profile_portrait_seed_prompt(&profile_with("小雪")).expect("prompt");
        assert!(prompt.contains("小雪"));
        assert!(prompt.contains("黑发"));
        assert!(prompt.contains("base_attributes"));
    }
}
