use super::shared::{non_empty_trimmed, truncate_chars};
use crate::state::AppState;
use a4n_core::models::character::{character_state_index, protagonist_profile};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::{HashMap, HashSet};

const RECENT_CHARACTER_STATE_ROWS: u64 = 80;
const CHARACTER_STATE_CONTEXT_LIMIT: usize = 8;

pub(super) async fn recent_character_state_context_lines(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<String>, String> {
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("构建角色状态上下文失败：查询角色档案失败: {err}"))?;
    if profiles.is_empty() {
        return Ok(Vec::new());
    }

    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    let profile_by_id = profiles
        .iter()
        .map(|profile| (profile.id, profile.clone()))
        .collect::<HashMap<_, _>>();

    let states = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids))
        .filter(character_state_index::Column::ChapterNumber.lt(chapter_number))
        .order_by_desc(character_state_index::Column::ChapterNumber)
        .limit(RECENT_CHARACTER_STATE_ROWS)
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("构建角色状态上下文失败：查询角色状态失败: {err}"))?;

    let mut seen_profiles = HashSet::new();
    let mut lines = Vec::new();
    for state_index in states {
        if !seen_profiles.insert(state_index.profile_id) {
            continue;
        }
        let Some(profile) = profile_by_id.get(&state_index.profile_id) else {
            continue;
        };
        if let Some(line) = character_state_context_line(&state_index, profile) {
            lines.push(line);
        }
        if lines.len() >= CHARACTER_STATE_CONTEXT_LIMIT {
            break;
        }
    }
    for profile in &profiles {
        if lines.len() >= CHARACTER_STATE_CONTEXT_LIMIT {
            break;
        }
        if seen_profiles.contains(&profile.id) {
            continue;
        }
        if let Some(line) = character_profile_context_line(profile, chapter_number) {
            lines.push(line);
        }
    }
    Ok(lines)
}

pub(super) fn character_state_context_line(
    state_index: &character_state_index::Model,
    profile: &protagonist_profile::Model,
) -> Option<String> {
    let mut details = Vec::new();
    if let Some(summary) = non_empty_trimmed(&state_index.state_summary) {
        details.push(truncate_chars(summary, 180));
    }
    if let Some(location) = state_index
        .current_location
        .as_deref()
        .and_then(non_empty_trimmed)
    {
        details.push(format!("位置: {}", truncate_chars(location, 60)));
    }
    if let Some(emotion) = state_index
        .emotional_state
        .as_deref()
        .and_then(non_empty_trimmed)
    {
        details.push(format!("情绪: {}", truncate_chars(emotion, 60)));
    }
    if let Some(goal) = state_index
        .current_goal
        .as_deref()
        .and_then(non_empty_trimmed)
    {
        details.push(format!("目标: {}", truncate_chars(goal, 100)));
    }
    if let Some(attributes) = character_profile_attribute_context(&state_index.key_attributes) {
        details.push(attributes);
    }

    if details.is_empty() {
        return None;
    }

    Some(format!(
        "- 第{}章后，{}：{}",
        state_index.chapter_number,
        profile.name,
        details.join("；")
    ))
}

pub(super) fn character_profile_context_line(
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> Option<String> {
    if profile.last_synced_chapter >= chapter_number {
        return None;
    }
    let attributes = character_profile_attribute_context(&profile.current_state)?;
    let chapter_label = if profile.last_synced_chapter > 0 {
        format!("第{}章后", profile.last_synced_chapter)
    } else {
        "初始档案".to_string()
    };
    Some(format!(
        "- {}，{}：{}",
        chapter_label, profile.name, attributes
    ))
}

fn character_profile_attribute_context(attributes: &serde_json::Value) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(text) = format_attribute_category(attributes, "explicit", "显性特征", 5) {
        parts.push(text);
    }
    if let Some(text) = format_attribute_category(attributes, "implicit", "性格特质", 3) {
        parts.push(text);
    }
    if let Some(text) = format_attribute_category(attributes, "social", "社会关系", 3) {
        parts.push(text);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("；"))
    }
}

fn format_attribute_category(
    attributes: &serde_json::Value,
    category: &str,
    label: &str,
    limit: usize,
) -> Option<String> {
    let storage_key = match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => return None,
    };
    let values = attributes
        .get(storage_key)
        .and_then(serde_json::Value::as_object)?;

    let rendered = values
        .iter()
        .filter_map(|(key, value)| {
            let key = key.trim();
            if key.is_empty() || value.is_null() {
                None
            } else {
                Some(format!("{}: {}", key, truncate_attribute_value(value, 30)))
            }
        })
        .take(limit)
        .collect::<Vec<_>>();

    if rendered.is_empty() {
        None
    } else {
        Some(format!("{label}: {}", rendered.join(", ")))
    }
}

fn truncate_attribute_value(value: &serde_json::Value, max_chars: usize) -> String {
    let text = match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(items) => items
            .iter()
            .take(3)
            .map(|item| match item {
                serde_json::Value::String(text) => truncate_chars(text, 15),
                other => truncate_chars(&other.to_string(), 15),
            })
            .collect::<Vec<_>>()
            .join(", "),
        serde_json::Value::Object(_) => value.to_string(),
        other => other.to_string(),
    };
    truncate_chars(&text, max_chars)
}
