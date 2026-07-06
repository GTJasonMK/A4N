use crate::state::AppState;
use a4n_core::models::{
    chapter::chapter,
    character::{protagonist_profile, protagonist_profile::Model as ProtagonistProfile},
};
pub(super) use a4n_services::character::profile_first_seen_chapter;
use serde_json::{Value, json};
use std::collections::BTreeSet;

use super::character_discovery::character_identity_key;

pub(super) fn is_cancelled_workflow_error(error: &str) -> bool {
    error.contains("已停止") || error.contains("已取消") || error.contains("cancelled")
}

pub(super) fn profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<&'static str> {
    if profile.last_synced_chapter == target_chapter {
        Some("already_synced_to_target")
    } else if profile.last_synced_chapter > target_chapter {
        Some("already_synced_past_target")
    } else if profile_first_seen_chapter(profile)
        .is_some_and(|first_seen| first_seen > target_chapter)
    {
        Some("target_before_first_seen")
    } else {
        None
    }
}

pub(super) fn analysis_profile_sync_skip_reason(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<&'static str> {
    if profile_first_seen_chapter(profile).is_some_and(|first_seen| first_seen > target_chapter) {
        Some("target_before_first_seen")
    } else {
        None
    }
}

pub(super) fn sync_skip_summary(profile: &ProtagonistProfile, target_chapter: i32) -> Value {
    sync_skip_summary_with_reason(
        profile,
        target_chapter,
        profile_sync_skip_reason(profile, target_chapter).unwrap_or("not_skipped"),
    )
}

pub(super) fn sync_skip_summary_with_reason(
    profile: &ProtagonistProfile,
    target_chapter: i32,
    reason: &str,
) -> Value {
    json!({
        "name": profile.name,
        "synced_chapter": profile.last_synced_chapter,
        "target_chapter": target_chapter,
        "first_seen_chapter": profile_first_seen_chapter(profile),
        "reason": reason,
    })
}

#[cfg(test)]
pub(super) fn character_sync_start_chapter(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
) -> Option<i32> {
    character_sync_range(profile, target_chapter, CharacterSyncRangeMode::Incremental)
        .map(|range| range.start_chapter)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CharacterSyncRangeMode {
    Incremental,
    AnalysisResyncFromTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CharacterSyncRange {
    pub(super) start_chapter: i32,
    pub(super) clear_to_chapter: i32,
}

pub(super) fn character_sync_range(
    profile: &protagonist_profile::Model,
    target_chapter: i32,
    mode: CharacterSyncRangeMode,
) -> Option<CharacterSyncRange> {
    let first_seen = profile_first_seen_chapter(profile).unwrap_or(1);
    if first_seen > target_chapter {
        return None;
    }

    match mode {
        CharacterSyncRangeMode::Incremental => {
            if profile_sync_skip_reason(profile, target_chapter).is_some() {
                return None;
            }
            let next_unsynced = profile.last_synced_chapter.max(0) + 1;
            Some(CharacterSyncRange {
                start_chapter: next_unsynced.max(first_seen),
                clear_to_chapter: target_chapter,
            })
        }
        CharacterSyncRangeMode::AnalysisResyncFromTarget => {
            if profile.last_synced_chapter >= target_chapter {
                Some(CharacterSyncRange {
                    start_chapter: target_chapter,
                    clear_to_chapter: profile.last_synced_chapter,
                })
            } else {
                let next_unsynced = profile.last_synced_chapter.max(0) + 1;
                Some(CharacterSyncRange {
                    start_chapter: next_unsynced.max(first_seen),
                    clear_to_chapter: target_chapter,
                })
            }
        }
    }
}

pub(super) fn analysis_sync_character_names(
    analysis_character_names: &BTreeSet<String>,
    discovery_character_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    analysis_character_names
        .iter()
        .chain(discovery_character_names.iter())
        .cloned()
        .collect()
}

pub(super) fn character_keys_from_names(names: &BTreeSet<String>) -> BTreeSet<String> {
    names
        .iter()
        .map(|name| character_identity_key(name))
        .filter(|key| !key.is_empty())
        .collect()
}

pub(super) fn character_names_from_chapter_analysis(analysis: &Value) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    if let Some(metadata) = analysis.get("metadata") {
        collect_character_name_values(metadata.get("characters"), &mut names);
    }

    if let Some(states) = analysis.get("character_states").and_then(Value::as_object) {
        for name in states.keys() {
            insert_analysis_character_name(name, &mut names);
        }
    }

    if let Some(events) = analysis.get("key_events").and_then(Value::as_array) {
        for event in events {
            collect_character_name_values(event.get("involved_characters"), &mut names);
        }
    }

    names
}

pub(super) fn character_names_from_discovery_result(result: &Value) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for field in ["created", "existing"] {
        let Some(items) = result.get(field).and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            collect_character_name_values(item.get("name"), &mut names);
            collect_character_name_values(item.get("aliases"), &mut names);
            if let Some(base_attributes) = item.get("base_attributes") {
                collect_character_attribute_aliases(base_attributes, &mut names);
            }
            if let Some(current_state) = item.get("current_state") {
                collect_character_attribute_aliases(current_state, &mut names);
            }
        }
    }
    names
}

pub(crate) async fn resolve_character_sync_target_chapter(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<i32, String> {
    let chapters = crate::workflow::character_sync_chapters(state, project_id).await?;
    validate_character_sync_target_chapter(&chapters, project_id, chapter_number)
}

pub(super) fn validate_character_sync_target_chapter(
    chapters: &[chapter::Model],
    project_id: i64,
    chapter_number: i32,
) -> Result<i32, String> {
    if chapter_number <= 0 {
        return Err("同步目标章节必须大于 0".to_string());
    }
    let latest_chapter = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .max()
        .ok_or_else(|| format!("项目还没有可同步章节: project_id={project_id}"))?;
    if chapter_number > latest_chapter {
        return Err(format!(
            "同步目标章节不存在: 第 {chapter_number} 章，当前最新章节为第 {latest_chapter} 章"
        ));
    }
    if chapters
        .iter()
        .any(|chapter| chapter.chapter_number == chapter_number)
    {
        Ok(chapter_number)
    } else {
        Err(format!(
            "第 {chapter_number} 章没有可同步正文，请先生成或导入正文"
        ))
    }
}

fn collect_character_attribute_aliases(attributes: &Value, names: &mut BTreeSet<String>) {
    if let Some(explicit) = attributes.get("explicit_attributes") {
        collect_character_name_values(explicit.get("aliases"), names);
    }
}

fn collect_character_name_values(value: Option<&Value>, names: &mut BTreeSet<String>) {
    match value {
        Some(Value::String(text)) => insert_analysis_character_name(text, names),
        Some(Value::Array(items)) => {
            for item in items {
                collect_character_name_values(Some(item), names);
            }
        }
        _ => {}
    }
}

fn insert_analysis_character_name(name: &str, names: &mut BTreeSet<String>) {
    let name = name.trim();
    if !name.is_empty() {
        names.insert(name.to_string());
    }
}
