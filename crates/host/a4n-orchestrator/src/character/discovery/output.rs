mod fields;

use std::collections::BTreeSet;

use a4n_domain::source_spans::AnchorQuoteIndex;
use a4n_domain::{chapter::ChapterRecord, text_anchor::TextAnchorRecord};
use a4n_domain::{json_contract::reject_unknown_fields, profile_text::normalize_importance_value};
use serde_json::{Value, json};

use crate::character::json_field_rules::{required_array, required_string, required_value};

use super::identity::discovered_character_keys;
use fields::{
    derive_discovery_summary, normalize_aliases, required_discovery_object, required_positive_i64,
    value_or_null,
};

pub(super) fn parse_character_discovery_output(
    content: &str,
    chapters: &[ChapterRecord],
    text_anchors: &[TextAnchorRecord],
) -> Result<Vec<Value>, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("角色发现结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err("角色发现结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(&value, "root", &["characters"])?;

    let mut seen = BTreeSet::new();
    let mut characters = Vec::new();
    let anchor_index = AnchorQuoteIndex::new(chapters, text_anchors);
    for (index, character) in required_array(&value, "characters")?.iter().enumerate() {
        let normalized = normalize_discovered_character(index, character, &anchor_index)?;
        let keys = discovered_character_keys(&normalized)?;
        if keys.iter().any(|key| seen.contains(key)) {
            return Err(format!("characters[{index}] 与前序角色重复"));
        }
        seen.extend(keys);
        characters.push(normalized);
    }
    Ok(characters)
}

fn normalize_discovered_character(
    index: usize,
    character: &Value,
    anchor_index: &AnchorQuoteIndex,
) -> Result<Value, String> {
    if !character.is_object() {
        return Err(format!("characters[{index}] 必须是 JSON 对象"));
    }
    let name = required_string(character, "name")?;
    let aliases = normalize_aliases(character, "aliases", &name)?;
    let importance = normalize_importance_value(&required_string(character, "importance")?)
        .map_err(|_| format!("characters[{index}].importance 必须是 primary/secondary/minor"))?;
    let role = required_string(character, "role")?;
    let explicit_attributes = required_discovery_object(character, "explicit_attributes")?;
    let implicit_attributes = required_discovery_object(character, "implicit_attributes")?;
    let social_attributes = required_discovery_object(character, "social_attributes")?;
    let evidence_path = format!("characters[{index}]");
    reject_unknown_fields(
        character,
        &evidence_path,
        &[
            "name",
            "aliases",
            "importance",
            "role",
            "explicit_attributes",
            "implicit_attributes",
            "social_attributes",
            "evidence_anchor_id",
        ],
    )?;
    let appearance = derive_discovery_summary(
        &explicit_attributes,
        &[
            "appearance",
            "外貌",
            "外观",
            "形象",
            "衣着",
            "服装",
            "发型",
            "体貌",
        ],
    );
    let background = derive_discovery_summary(
        &explicit_attributes,
        &["background", "背景", "出身", "来历", "经历", "职业"],
    )
    .or_else(|| {
        derive_discovery_summary(
            &implicit_attributes,
            &["background", "背景", "动机来源", "过往"],
        )
    });
    let personality = derive_discovery_summary(
        &implicit_attributes,
        &[
            "personality",
            "性格",
            "特质",
            "行为倾向",
            "行事风格",
            "心理",
        ],
    );
    let evidence_anchor_id = required_positive_i64(character, "evidence_anchor_id", index)?;
    let evidence_anchor = anchor_index.quote_with_chapter_number(
        &format!("{evidence_path}.evidence_anchor_id"),
        evidence_anchor_id,
    )?;
    Ok(json!({
        "name": name,
        "aliases": aliases,
        "first_seen_chapter": evidence_anchor.chapter_number,
        "latest_seen_chapter": evidence_anchor.chapter_number,
        "importance": importance,
        "role": role,
        "explicit_attributes": explicit_attributes,
        "implicit_attributes": implicit_attributes,
        "social_attributes": social_attributes,
        "appearance": value_or_null(appearance),
        "background": value_or_null(background),
        "personality": value_or_null(personality),
        "evidence_anchor_id": evidence_anchor_id,
        "evidence": evidence_anchor.quote,
    }))
}

pub(super) fn discovered_character_base_attributes(character: &Value) -> Result<Value, String> {
    let mut explicit = required_value(character, "explicit_attributes")?
        .as_object()
        .ok_or_else(|| "角色发现字段 explicit_attributes 必须是 JSON 对象".to_string())?
        .clone();
    let mut implicit = required_value(character, "implicit_attributes")?
        .as_object()
        .ok_or_else(|| "角色发现字段 implicit_attributes 必须是 JSON 对象".to_string())?
        .clone();
    let social = required_value(character, "social_attributes")?
        .as_object()
        .ok_or_else(|| "角色发现字段 social_attributes 必须是 JSON 对象".to_string())?
        .clone();

    explicit.insert(
        "role".to_string(),
        Value::String(required_string(character, "role")?),
    );
    explicit.insert(
        "aliases".to_string(),
        required_value(character, "aliases")?.clone(),
    );
    explicit.insert(
        "first_seen_chapter".to_string(),
        required_value(character, "first_seen_chapter")?.clone(),
    );
    explicit.insert(
        "latest_seen_chapter".to_string(),
        required_value(character, "latest_seen_chapter")?.clone(),
    );
    explicit.insert(
        "importance".to_string(),
        Value::String(required_string(character, "importance")?),
    );
    explicit.insert(
        "evidence".to_string(),
        Value::String(required_string(character, "evidence")?),
    );
    insert_optional_attribute(
        &mut explicit,
        "appearance",
        required_value(character, "appearance")?,
    );
    insert_optional_attribute(
        &mut explicit,
        "background",
        required_value(character, "background")?,
    );
    insert_optional_attribute(
        &mut implicit,
        "personality",
        required_value(character, "personality")?,
    );
    Ok(json!({
        "explicit_attributes": explicit,
        "implicit_attributes": implicit,
        "social_attributes": social,
    }))
}

fn insert_optional_attribute(
    attributes: &mut serde_json::Map<String, Value>,
    key: &str,
    value: &Value,
) {
    if !value.is_null() && !attributes.contains_key(key) {
        attributes.insert(key.to_string(), value.clone());
    }
}

pub(super) fn character_discovery_parsed_output_json(discovered: &[Value]) -> Value {
    json!({"characters": discovered})
}
