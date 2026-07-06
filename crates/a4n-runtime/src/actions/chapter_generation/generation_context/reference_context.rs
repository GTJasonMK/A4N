use super::push_optional_text;
use crate::AppState;
use a4n_core::models::{
    chapter::chapter_outline,
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;

pub(crate) async fn resolve_generation_reference_context(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
) -> Result<Option<String>, String> {
    build_generation_reference_context(state, project_id, chapter_number, outline)
        .await
        .map_err(|err| format!("构建章节生成关键参考失败: {err}"))
}

async fn build_generation_reference_context(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
) -> Result<Option<String>, String> {
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_desc(blueprint_relationship::Column::Strength)
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"))
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .limit(20)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let protagonist_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut sections = Vec::new();
    if let Some(blueprint) = blueprint.as_ref() {
        sections.push(format_blueprint_core_section(blueprint, &characters));
    } else if !characters.is_empty() {
        let names = characters
            .iter()
            .map(|character| character.name.as_str())
            .collect::<Vec<_>>();
        sections.push(format!("## 核心设定\n角色名单: {}", names.join("、")));
    }

    let involved_names = outline_character_names(outline);
    let involved_characters = select_generation_characters(&characters, &involved_names);
    let relevant_relationships = select_generation_relationships(&relationships, &involved_names);
    let pending_foreshadowing =
        select_generation_foreshadowing(&foreshadowing, chapter_number, &involved_names);

    let mut reference_lines = Vec::new();
    if !involved_characters.is_empty() {
        reference_lines.push("### 涉及角色".to_string());
        reference_lines.extend(
            involved_characters
                .iter()
                .map(format_character_reference_line),
        );
    }
    if !relevant_relationships.is_empty() {
        if !reference_lines.is_empty() {
            reference_lines.push(String::new());
        }
        reference_lines.push("### 角色关系".to_string());
        reference_lines.extend(
            relevant_relationships
                .iter()
                .map(format_relationship_reference_line),
        );
    }
    if let Some(profile_section) =
        format_protagonist_profile_reference(&protagonist_profiles, &involved_names)
    {
        if !reference_lines.is_empty() {
            reference_lines.push(String::new());
        }
        reference_lines.extend(profile_section);
    }
    if !pending_foreshadowing.is_empty() {
        if !reference_lines.is_empty() {
            reference_lines.push(String::new());
        }
        reference_lines.push("### 待回收伏笔".to_string());
        reference_lines.extend(
            pending_foreshadowing
                .iter()
                .map(format_foreshadowing_reference_line),
        );
    }
    if !reference_lines.is_empty() {
        sections.push(format!("## 关键参考\n{}", reference_lines.join("\n")));
    }

    if let Some(world_setting) = blueprint
        .as_ref()
        .and_then(|blueprint| blueprint.world_setting.as_ref())
        .filter(|value| is_meaningful_json_value(value))
    {
        let text = serde_json::to_string_pretty(world_setting)
            .expect("serde_json::Value pretty serialization must succeed");
        sections.push(format!(
            "## 补充参考\n世界观设定:\n{}",
            truncate_text_chars(&text, 900)
        ));
    }

    if sections.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sections.join("\n\n")))
    }
}

fn format_blueprint_core_section(
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

pub(super) fn select_generation_characters<'a>(
    characters: &'a [blueprint_character::Model],
    involved_names: &[String],
) -> Vec<&'a blueprint_character::Model> {
    let selected = if involved_names.is_empty() {
        characters
            .iter()
            .filter(|character| {
                matches!(
                    character.character_type.as_str(),
                    "protagonist" | "antagonist" | "supporting"
                )
            })
            .collect::<Vec<_>>()
    } else {
        characters
            .iter()
            .filter(|character| involved_names.iter().any(|name| name == &character.name))
            .collect::<Vec<_>>()
    };

    if selected.is_empty() {
        characters.iter().collect()
    } else {
        selected
    }
}

fn select_generation_relationships<'a>(
    relationships: &'a [blueprint_relationship::Model],
    involved_names: &[String],
) -> Vec<&'a blueprint_relationship::Model> {
    if involved_names.is_empty() {
        return relationships.iter().collect();
    }
    relationships
        .iter()
        .filter(|relationship| {
            involved_names
                .iter()
                .any(|name| name == &relationship.character_a || name == &relationship.character_b)
        })
        .collect()
}

pub(super) fn select_generation_foreshadowing<'a>(
    foreshadowing: &'a [foreshadowing_index::Model],
    chapter_number: i32,
    involved_names: &[String],
) -> Vec<&'a foreshadowing_index::Model> {
    foreshadowing
        .iter()
        .filter(|item| {
            let relevant_time =
                item.setup_chapter < chapter_number || item.payoff_chapter == Some(chapter_number);
            let unresolved = match item.payoff_chapter {
                Some(payoff) => payoff >= chapter_number,
                None => true,
            };
            relevant_time
                && unresolved
                && (involved_names.is_empty()
                    || item.related_characters.as_ref().is_some_and(|value| {
                        let related = extract_character_names_from_value(value);
                        related
                            .iter()
                            .any(|name| involved_names.iter().any(|involved| involved == name))
                    }))
        })
        .collect()
}

pub(super) fn format_character_reference_line(character: &&blueprint_character::Model) -> String {
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

fn format_relationship_reference_line(relationship: &&blueprint_relationship::Model) -> String {
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

fn format_protagonist_profile_reference(
    profiles: &[protagonist_profile::Model],
    involved_names: &[String],
) -> Option<Vec<String>> {
    let mut selected = profiles
        .iter()
        .filter(|profile| involved_names.iter().any(|name| name == &profile.name))
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

pub(super) fn outline_character_names(outline: Option<&chapter_outline::Model>) -> Vec<String> {
    match outline.and_then(|outline| outline.characters_involved.as_ref()) {
        Some(value) => extract_character_names_from_value(value),
        None => Vec::new(),
    }
}

pub(super) fn format_foreshadowing_reference_line(item: &&foreshadowing_index::Model) -> String {
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

fn extract_character_names_from_value(value: &Value) -> Vec<String> {
    let mut names = Vec::new();
    collect_character_names(value, &mut names);
    names.sort();
    names.dedup();
    names
}

fn collect_character_names(value: &Value, names: &mut Vec<String>) {
    match value {
        Value::String(text) => push_name(names, text),
        Value::Array(items) => {
            for item in items {
                collect_character_names(item, names);
            }
        }
        Value::Object(map) => {
            for key in ["name", "character", "character_name", "人物", "角色"] {
                if let Some(text) = map.get(key).and_then(Value::as_str) {
                    push_name(names, text);
                }
            }
        }
        _ => {}
    }
}

fn push_name(names: &mut Vec<String>, text: &str) {
    let text = text.trim();
    if !text.is_empty() {
        names.push(text.to_string());
    }
}

fn push_kv(parts: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) {
        parts.push(format!("{key}: {}", truncate_text_chars(value, 80)));
    }
}

pub(super) fn truncate_text_chars(text: &str, max_chars: usize) -> String {
    let mut iter = text.trim().chars();
    let truncated = iter.by_ref().take(max_chars).collect::<String>();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

pub(super) fn compact_json_text(value: &Value, max_chars: usize) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).expect("serde_json::Value serialization must succeed"),
    };
    truncate_text_chars(&text, max_chars)
}

pub(super) fn is_meaningful_json_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(_) | Value::Number(_) | Value::String(_) => true,
        Value::Array(items) => !items.is_empty() && items.iter().any(is_meaningful_json_value),
        Value::Object(map) => !map.is_empty() && map.values().any(is_meaningful_json_value),
    }
}
