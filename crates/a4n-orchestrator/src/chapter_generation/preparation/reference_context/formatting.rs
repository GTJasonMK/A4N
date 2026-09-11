use std::collections::HashSet;

use super::super::outline::push_optional_text;
use super::json::{compact_json_text, is_meaningful_json_value, truncate_text_chars};
use a4n_db_models::models::{
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use serde_json::Value;

pub(super) fn format_blueprint_core_section(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
) -> String {
    let mut lines = vec!["## 核心设定".to_string()];
    let mut basics = Vec::new();
    push_kv(&mut basics, "题材", Some(blueprint.genre.as_str()));
    push_kv(&mut basics, "风格", blueprint.style.as_deref());
    push_kv(&mut basics, "基调", blueprint.tone.as_deref());
    if !basics.is_empty() {
        lines.push(basics.join(" | "));
    }
    push_optional_text(
        &mut lines,
        "故事",
        blueprint.one_sentence_summary.as_deref(),
    );
    let names = characters
        .iter()
        .map(|character| character.name.as_str())
        .collect::<Vec<_>>();
    if !names.is_empty() {
        lines.push(format!("角色名单: {}", names.join("、")));
    }
    lines.join("\n")
}

pub(in crate::chapter_generation::preparation) fn format_character_reference_line(
    character: &&blueprint_character::Model,
) -> String {
    let mut parts = vec![format!("类型: {}", character.character_type)];
    push_kv(&mut parts, "性别", character.gender.as_deref());
    push_kv(&mut parts, "年龄", character.age.as_deref());
    push_kv(&mut parts, "外貌", character.appearance.as_deref());
    push_kv(&mut parts, "背景", character.background.as_deref());
    push_kv(&mut parts, "性格", character.personality.as_deref());
    push_kv(&mut parts, "动机", character.motivation.as_deref());
    push_kv(&mut parts, "弧光", character.arc_summary.as_deref());
    push_kv(
        &mut parts,
        "与主角关系",
        character.relationship_to_protagonist.as_deref(),
    );
    format!("- {}：{}", character.name, parts.join("；"))
}

pub(super) fn format_relationship_reference_line(
    relationship: &&blueprint_relationship::Model,
) -> String {
    let mut suffix = Vec::new();
    push_kv(
        &mut suffix,
        "类型",
        Some(relationship.relationship_type.as_str()),
    );
    if let Some(strength) = relationship.strength {
        suffix.push(format!("强度: {strength}"));
    }
    push_kv(&mut suffix, "描述", relationship.description.as_deref());
    format!(
        "- {} ↔ {}：{}",
        relationship.character_a,
        relationship.character_b,
        suffix.join("；")
    )
}

pub(super) fn format_protagonist_profile_reference(
    profiles: &[protagonist_profile::Model],
    involved_names: &[String],
) -> Option<Vec<String>> {
    let involved_names = involved_names
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut selected = profiles
        .iter()
        .filter(|profile| involved_names.contains(profile.name.as_str()))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        selected = profiles.iter().collect::<Vec<_>>();
    }
    if selected.is_empty() {
        return None;
    }

    let mut lines = vec![
        "### 主角档案约束".to_string(),
        "请确保角色行为、语言和选择与以下档案保持一致。".to_string(),
    ];
    for profile in selected {
        let mut parts = Vec::new();
        push_kv(&mut parts, "性别", profile.gender.as_deref());
        push_kv(&mut parts, "年龄", profile.age.as_deref());
        push_kv(&mut parts, "外貌", profile.appearance.as_deref());
        push_kv(&mut parts, "背景", profile.background.as_deref());
        push_kv(&mut parts, "人格", profile.personality.as_deref());
        push_profile_json_kv(&mut parts, "基础属性", &profile.base_attributes, 160);
        push_profile_json_kv(&mut parts, "当前状态", &profile.current_state, 180);
        if let Some(arc) = profile.character_arc.as_ref() {
            push_profile_json_kv(&mut parts, "角色弧光", arc, 160);
        }
        if let Some(voice) = profile.voice_fingerprint.as_ref() {
            push_profile_json_kv(&mut parts, "声音指纹", voice, 120);
        }
        if !parts.is_empty() {
            lines.push(format!("- {}：{}", profile.name, parts.join("；")));
        }
    }

    if lines.len() > 2 { Some(lines) } else { None }
}

fn push_profile_json_kv(parts: &mut Vec<String>, key: &str, value: &Value, max_chars: usize) {
    if is_meaningful_json_value(value) {
        parts.push(format!("{key}: {}", compact_json_text(value, max_chars)));
    }
}

pub(in crate::chapter_generation::preparation) fn format_foreshadowing_reference_line(
    item: &&foreshadowing_index::Model,
) -> String {
    match item.payoff_chapter {
        Some(chapter) => format!(
            "- [重要度{}] 第{}章埋设，预计第{}章回收：{}",
            item.importance, item.setup_chapter, chapter, item.description
        ),
        None => format!(
            "- [重要度{}] 第{}章埋设：{}",
            item.importance, item.setup_chapter, item.description
        ),
    }
}

fn push_kv(parts: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("{key}: {}", truncate_text_chars(value, 80)));
    }
}
