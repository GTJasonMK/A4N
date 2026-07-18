mod fields;

use std::collections::BTreeSet;

use serde_json::{Value, json};

use a4n_core::models::chapter::chapter;
use a4n_core::models::project::text_anchor;
use a4n_core::profile_text::normalize_importance_value;

use crate::actions::json_contract::reject_unknown_fields;
use crate::actions::text_anchor_contract::{
    anchor_quote_with_chapter_number, reject_deprecated_output_fields,
};

use super::super::json_utils::{required_array, required_string};
use super::identity::discovered_character_keys;
use fields::{
    derive_discovery_summary, normalize_aliases, required_discovery_object, required_positive_i64,
    value_or_null,
};

pub(in crate::actions::character_ai) fn parse_character_discovery_output(
    content: &str,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
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
    chapters: &[chapter::Model],
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
    let evidence_anchor_id = required_positive_i64(character, "evidence_anchor_id", index)?;
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
