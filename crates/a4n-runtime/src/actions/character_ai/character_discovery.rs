use crate::actions::json_contract::reject_unknown_fields;
use crate::actions::llm_request::PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT;
use crate::actions::text_anchor_contract::{
    anchor_quote_with_chapter_number, reject_deprecated_output_fields,
};
use a4n_core::models::character::protagonist_profile;
use a4n_core::models::project::text_anchor;
use a4n_engine::prompt::request::build_json_task_user_message;
use a4n_services::profile_text::normalize_importance_value;
use a4n_services::text_anchor::text_anchor_prompt_values;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::json_utils::{
    attribute_counts, category_attributes, required_array, required_string, required_value,
};

#[cfg(test)]
pub(super) fn build_character_discovery_user_prompt(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[a4n_core::models::chapter::chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<String, String> {
    let payload = character_discovery_payload(json!({
        "existing_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors)
    }));
    build_character_discovery_prompt_from_payload(payload)
}

pub(super) fn build_character_discovery_manifest_user_prompt() -> Result<String, String> {
    let payload = character_discovery_payload(json!({
        "context_refs": {
            "source": format!("稳定上下文包 {PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT}")
        }
    }));
    build_character_discovery_prompt_from_payload(payload)
}

pub(super) fn character_discovery_stable_context(
    existing_profiles: &[Value],
    chapter_data: &[Value],
    chapters: &[a4n_core::models::chapter::chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Value {
    json!({
        "existing_profiles": existing_profiles,
        "chapters": chapter_data,
        "text_anchors": text_anchor_prompt_values(chapters, text_anchors)
    })
}

fn character_discovery_payload(input: Value) -> Value {
    json!({
        "task": {
            "name": "character_discovery",
            "instruction": "从本次输入章节中识别真实人物角色；已有角色档案只用于同一人物判定，不要求模型自行过滤已存在角色"
        },
        "input": input,
        "output_contract": {
            "format": "json_object",
            "required_root_field": "characters",
            "character_scope": "返回本次输入章节中识别出的角色。已有角色也可以返回，后端会归入 existing，不会重复创建。",
            "characters": "array",
            "size_limits": {
                "max_characters": 24,
                "aliases_max_items": 4,
                "attribute_object_max_keys_each": 4,
                "role_max_chars": 12,
                "evidence_anchor_id_count": 1
            },
            "character_required_fields": [
                "name",
                "aliases",
                "importance",
                "role",
                "explicit_attributes",
                "implicit_attributes",
                "social_attributes",
                "evidence_anchor_id"
            ],
            "system_derived_fields": [
                "appearance",
                "background",
                "personality",
                "first_seen_chapter",
                "latest_seen_chapter",
                "evidence"
            ],
            "importance_values": ["primary", "secondary", "minor"],
            "chapter_scope": "后端会根据 evidence_anchor_id 所在章节推导 first_seen_chapter 和 latest_seen_chapter；不要输出这两个字段",
            "strict_rules": [
                "只返回可解析的完整 JSON 对象，不要 Markdown 或解释文字",
                "字段要短，属性只保留身份、关系、关键状态等最能区分角色的信息",
                "不要输出 evidence 或 evidence_span_id 字段；必须输出 evidence_anchor_id，并且 ID 必须来自 input.text_anchors[].id",
                "不要输出 first_seen_chapter 或 latest_seen_chapter；系统会从 evidence_anchor_id 推导",
                "不要输出 appearance、background、personality；系统会从三类属性对象中提炼摘要",
                "evidence_anchor_id 只选择最能证明角色首次出场或身份的一个 text_anchor；不要自己抄写原文，不要输出省略号证据",
                "如角色很多，优先保留有姓名、有独立行为、后续可能复用的角色"
            ]
        }
    })
}

fn build_character_discovery_prompt_from_payload(payload: Value) -> Result<String, String> {
    build_json_task_user_message(
        "执行角色发现",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
    .map_err(|e| format!("角色发现上下文序列化失败: {e}"))
}

pub(super) fn existing_profile_for_prompt(
    profile: &protagonist_profile::Model,
) -> Result<Value, String> {
    Ok(json!({
        "name": profile.name,
        "aliases": profile_aliases(profile),
        "gender": optional_profile_text(profile.gender.as_deref()),
        "age": optional_profile_text(profile.age.as_deref()),
        "appearance": optional_profile_text(profile.appearance.as_deref()),
        "background": optional_profile_text(profile.background.as_deref()),
        "personality": optional_profile_text(profile.personality.as_deref()),
        "base_explicit_attributes": explicit_attributes_for_identity(
            &profile.base_attributes,
            "base_attributes",
        )?,
        "current_explicit_attributes": explicit_attributes_for_identity(
            &profile.current_state,
            "current_state",
        )?,
        "last_synced_chapter": profile.last_synced_chapter,
        "attribute_counts": attribute_counts(&profile.current_state)?,
    }))
}

pub(super) fn profile_for_result(profile: &protagonist_profile::Model) -> Result<Value, String> {
    Ok(json!({
        "id": profile.id,
        "project_id": profile.project_id,
        "name": profile.name,
        "gender": profile.gender,
        "age": profile.age,
        "base_attributes": profile.base_attributes,
        "current_state": profile.current_state,
        "character_arc": profile.character_arc,
        "voice_fingerprint": profile.voice_fingerprint,
        "appearance": profile.appearance,
        "background": profile.background,
        "personality": profile.personality,
        "last_synced_chapter": profile.last_synced_chapter,
        "attribute_counts": attribute_counts(&profile.current_state)?,
        "created_at": profile.created_at.to_string(),
        "updated_at": profile.updated_at.to_string(),
    }))
}

pub(super) fn existing_character_keys(profiles: &[protagonist_profile::Model]) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for profile in profiles {
        keys.extend(profile_character_keys(profile));
    }
    keys.retain(|key| !key.is_empty());
    keys
}

pub(super) fn profile_character_keys(profile: &protagonist_profile::Model) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    keys.insert(character_identity_key(&profile.name));
    for alias in profile_aliases(profile) {
        keys.insert(character_identity_key(&alias));
    }
    keys.retain(|key| !key.is_empty());
    keys
}

fn profile_aliases(profile: &protagonist_profile::Model) -> Vec<String> {
    aliases_from_attributes(&profile.base_attributes)
        .into_iter()
        .chain(aliases_from_attributes(&profile.current_state))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn aliases_from_attributes(attributes: &Value) -> Vec<String> {
    let Some(aliases) = attributes
        .get("explicit_attributes")
        .and_then(Value::as_object)
        .and_then(|explicit| explicit.get("aliases"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    aliases
        .iter()
        .filter_map(|alias| alias.as_str().map(str::trim))
        .filter(|alias| !alias.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn optional_profile_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn explicit_attributes_for_identity(attributes: &Value, path: &str) -> Result<Value, String> {
    category_attributes(attributes, "explicit")
        .map_err(|err| format!("已有角色档案 {path} 无法作为角色发现上下文：{err}"))
}

pub(super) fn discovered_character_keys(character: &Value) -> Result<Vec<String>, String> {
    let mut keys = BTreeSet::new();
    keys.insert(character_identity_key(&required_string(character, "name")?));
    for alias in required_array(character, "aliases")? {
        let alias = alias
            .as_str()
            .ok_or_else(|| "角色发现字段 aliases 必须是字符串数组".to_string())?
            .trim();
        if !alias.is_empty() {
            keys.insert(character_identity_key(alias));
        }
    }
    keys.retain(|key| !key.is_empty());
    Ok(keys.into_iter().collect())
}

pub(super) fn character_identity_key(name: &str) -> String {
    name.trim().to_lowercase()
}

pub(super) fn parse_character_discovery_output(
    content: &str,
    chapters: &[a4n_core::models::chapter::chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<Value>, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|e| {
        let mut message = format!("角色发现结果必须是 JSON 对象: {e}");
        if e.is_eof() {
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
    for (index, character) in required_array(&value, "characters")?.iter().enumerate() {
        let normalized = normalize_discovered_character(index, character, chapters, text_anchors)?;
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
    chapters: &[a4n_core::models::chapter::chapter::Model],
    text_anchors: &[text_anchor::Model],
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
    reject_deprecated_output_fields(
        character,
        &evidence_path,
        &["evidence", "evidence_span_id"],
        "evidence_anchor_id",
    )?;
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
    let evidence_anchor_id = required_i64(character, "evidence_anchor_id", index)?;
    let evidence_anchor = anchor_quote_with_chapter_number(
        &format!("{evidence_path}.evidence_anchor_id"),
        evidence_anchor_id,
        chapters,
        text_anchors,
    )?;
    let first_seen_chapter = evidence_anchor.chapter_number;
    let latest_seen_chapter = evidence_anchor.chapter_number;
    let evidence = evidence_anchor.quote;

    Ok(json!({
        "name": name,
        "aliases": aliases,
        "first_seen_chapter": first_seen_chapter,
        "latest_seen_chapter": latest_seen_chapter,
        "importance": importance,
        "role": role,
        "explicit_attributes": explicit_attributes,
        "implicit_attributes": implicit_attributes,
        "social_attributes": social_attributes,
        "appearance": value_or_null(appearance),
        "background": value_or_null(background),
        "personality": value_or_null(personality),
        "evidence_anchor_id": evidence_anchor_id,
        "evidence": evidence,
    }))
}

fn required_i64(json: &Value, key: &str, index: usize) -> Result<i64, String> {
    let number = json
        .get(key)
        .ok_or_else(|| format!("characters[{index}] 缺少字段: {key}"))?
        .as_i64()
        .ok_or_else(|| format!("characters[{index}].{key} 必须是 JSON 整数数字"))?;
    if number <= 0 {
        return Err(format!("characters[{index}].{key} 必须大于 0"));
    }
    Ok(number)
}

fn normalize_aliases(json: &Value, key: &str, name: &str) -> Result<Value, String> {
    let values = required_array(json, key)?;
    let mut normalized = Vec::with_capacity(values.len());
    let mut seen = BTreeSet::new();
    let name_key = character_identity_key(name);
    for value in values {
        let text = value
            .as_str()
            .ok_or_else(|| format!("角色发现字段 {key} 必须是字符串数组"))?
            .trim();
        if text.is_empty() {
            continue;
        }
        let alias_key = character_identity_key(text);
        if alias_key == name_key {
            continue;
        }
        if seen.insert(alias_key) {
            normalized.push(Value::String(text.to_string()));
        }
    }
    Ok(Value::Array(normalized))
}

fn required_discovery_object(json: &Value, key: &str) -> Result<Value, String> {
    json.get(key)
        .and_then(Value::as_object)
        .map(|object| Value::Object(object.clone()))
        .ok_or_else(|| format!("角色发现字段 {key} 必须是 JSON 对象"))
}

fn value_or_null(value: Option<String>) -> Value {
    value.map(Value::String).unwrap_or(Value::Null)
}

fn derive_discovery_summary(attributes: &Value, candidate_keys: &[&str]) -> Option<String> {
    let object = attributes.as_object()?;
    candidate_keys.iter().find_map(|key| {
        object
            .get(*key)
            .and_then(summary_text_from_value)
            .filter(|text| !text.is_empty())
    })
}

fn summary_text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.trim().to_string()),
        Value::Array(items) => {
            let parts = items
                .iter()
                .filter_map(summary_text_from_value)
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("、"))
        }
        Value::Object(object) => {
            let parts = object
                .iter()
                .filter_map(|(key, value)| {
                    summary_text_from_value(value).map(|text| format!("{key}:{text}"))
                })
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("；"))
        }
        _ => None,
    }
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
