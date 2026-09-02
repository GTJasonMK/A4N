use std::collections::{HashMap, HashSet};

use a4n_core::models::{
    character::{
        attribute_event::{self, SOURCE_KIND_AI_SYNC},
        protagonist_deletion_mark, protagonist_profile,
    },
    generation::{generation_run, generation_run_target},
    project::blueprint_character,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;

use crate::character::attributes::{apply_attribute_to_state, attribute_value_in_state};
use crate::character::{CharacterDbError, validate_timeline_source_kind};

#[derive(Debug, Clone, Default)]
pub(in crate::character) struct CharacterAttributeOwnership {
    protected_from_chapter: HashMap<String, HashMap<String, i32>>,
}

impl CharacterAttributeOwnership {
    pub(in crate::character) fn is_protected(
        &self,
        chapter_number: i32,
        category: &str,
        key: &str,
    ) -> bool {
        self.protected_from_chapter
            .get(category)
            .and_then(|attributes| attributes.get(key))
            .is_some_and(|protected_from| *protected_from <= chapter_number)
    }

    fn protect(&mut self, chapter_number: i32, category: &str, key: &str) {
        self.protected_from_chapter
            .entry(category.to_string())
            .or_default()
            .entry(key.to_string())
            .and_modify(|current| *current = (*current).min(chapter_number))
            .or_insert(chapter_number);
    }
}

pub(in crate::character) async fn character_attribute_ownership<C>(
    db: &C,
    profile: &protagonist_profile::Model,
) -> Result<CharacterAttributeOwnership, CharacterDbError>
where
    C: ConnectionTrait,
{
    let (events_result, deletion_marks_result, base_attributes_result) = futures::join!(
        attribute_events(db, profile.id),
        protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile.id))
            .all(db),
        profile_base_attributes_are_ai_generated(db, profile),
    );
    let events = events_result?;
    let deletion_marks = deletion_marks_result?;
    let base_attributes_are_ai_generated = base_attributes_result?;
    Ok(character_attribute_ownership_from_records(
        profile,
        &events,
        &deletion_marks,
        base_attributes_are_ai_generated,
    ))
}

/// 从初始档案与正式事件重建指定章节末的角色状态。
pub async fn replay_profile_state_through_chapter<C>(
    db: &C,
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> Result<Value, CharacterDbError>
where
    C: ConnectionTrait,
{
    if chapter_number < 0 {
        return Err(CharacterDbError::InvalidArgument(
            "角色状态重放截止章节不能小于 0".to_string(),
        ));
    }
    let (events_result, deletion_marks_result, base_attributes_result) = futures::join!(
        attribute_events(db, profile.id),
        protagonist_deletion_mark::Entity::find()
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile.id))
            .all(db),
        profile_base_attributes_are_ai_generated(db, profile),
    );
    let events = events_result?;
    let deletion_marks = deletion_marks_result?;
    let base_attributes_are_ai_generated = base_attributes_result?;
    let ownership = character_attribute_ownership_from_records(
        profile,
        &events,
        &deletion_marks,
        base_attributes_are_ai_generated,
    );
    replay_profile_state_from_records(profile, &events, chapter_number, &ownership)
}

async fn attribute_events<C>(
    db: &C,
    profile_id: i64,
) -> Result<Vec<attribute_event::Model>, CharacterDbError>
where
    C: ConnectionTrait,
{
    attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile_id))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id)
        .all(db)
        .await
        .map_err(CharacterDbError::Database)
}

async fn profile_base_attributes_are_ai_generated<C>(
    db: &C,
    profile: &protagonist_profile::Model,
) -> Result<bool, CharacterDbError>
where
    C: ConnectionTrait,
{
    let profile_key = crate::project::normalized_blueprint_identity(&profile.name);
    let blueprint_owned = blueprint_character::Entity::find()
        .select_only()
        .column(blueprint_character::Column::Name)
        .filter(blueprint_character::Column::ProjectId.eq(profile.project_id))
        .into_tuple::<String>()
        .all(db)
        .await?
        .into_iter()
        .any(|name| crate::project::normalized_blueprint_identity(&name) == profile_key);
    if blueprint_owned {
        return Ok(false);
    }

    let run_ids = generation_run_target::Entity::find()
        .select_only()
        .column(generation_run_target::Column::GenerationRunId)
        .filter(generation_run_target::Column::TargetTable.eq("protagonist_profile"))
        .filter(generation_run_target::Column::TargetId.eq(profile.id))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if run_ids.is_empty() {
        return Ok(false);
    }
    Ok(generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .filter(generation_run::Column::Id.is_in(run_ids))
        .filter(generation_run::Column::Workflow.eq("character_discovery"))
        .filter(generation_run::Column::Status.eq("completed"))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some())
}

fn character_attribute_ownership_from_records(
    profile: &protagonist_profile::Model,
    events: &[attribute_event::Model],
    deletion_marks: &[protagonist_deletion_mark::Model],
    base_attributes_are_ai_generated: bool,
) -> CharacterAttributeOwnership {
    let mut ownership = CharacterAttributeOwnership::default();
    if !base_attributes_are_ai_generated {
        for (category, key, _) in attribute_entries(&profile.base_attributes) {
            ownership.protect(1, &category, &key);
        }
    }

    let event_attributes = events
        .iter()
        .map(|event| (event.category.as_str(), event.key.as_str()))
        .collect::<HashSet<_>>();
    for (category, key, value) in attribute_entries(&profile.current_state) {
        if event_attributes.contains(&(category.as_str(), key.as_str())) {
            continue;
        }
        if attribute_value_in_state(&profile.base_attributes, &category, &key).as_ref()
            != Some(&value)
        {
            ownership.protect(1, &category, &key);
        }
    }

    for event in events {
        if event.source_kind != SOURCE_KIND_AI_SYNC {
            ownership.protect(event.chapter_number, &event.category, &event.key);
        }
    }
    for mark in deletion_marks {
        if mark.source_kind != SOURCE_KIND_AI_SYNC {
            ownership.protect(mark.chapter_number, &mark.category, &mark.key);
        }
    }
    ownership
}

fn replay_profile_state_from_records(
    profile: &protagonist_profile::Model,
    events: &[attribute_event::Model],
    chapter_number: i32,
    ownership: &CharacterAttributeOwnership,
) -> Result<Value, CharacterDbError> {
    let mut state = profile.base_attributes.clone();
    let event_attributes = events
        .iter()
        .map(|event| (event.category.as_str(), event.key.as_str()))
        .collect::<HashSet<_>>();
    for (category, key, value) in attribute_entries(&profile.current_state) {
        if !event_attributes.contains(&(category.as_str(), key.as_str()))
            && attribute_value_in_state(&profile.base_attributes, &category, &key).as_ref()
                != Some(&value)
        {
            apply_attribute_to_state(&mut state, &category, &key, "modify", value)?;
        }
    }

    for event in events
        .iter()
        .filter(|event| event.chapter_number <= chapter_number)
    {
        validate_timeline_source_kind(&event.source_kind)?;
        if !matches!(event.operation.as_str(), "add" | "modify" | "delete") {
            return Err(CharacterDbError::InvalidState {
                current: format!(
                    "属性事件 {} 的 operation 无效: {}",
                    event.id, event.operation
                ),
                operation: "重放角色属性事件".to_string(),
            });
        }
        if event.source_kind == SOURCE_KIND_AI_SYNC
            && ownership.is_protected(event.chapter_number, &event.category, &event.key)
        {
            continue;
        }
        apply_attribute_to_state(
            &mut state,
            &event.category,
            &event.key,
            &event.operation,
            event.new_value.clone(),
        )?;
    }
    Ok(state)
}

fn attribute_entries(state: &Value) -> Vec<(String, String, Value)> {
    [
        ("explicit", "explicit_attributes"),
        ("implicit", "implicit_attributes"),
        ("social", "social_attributes"),
    ]
    .into_iter()
    .flat_map(|(category, storage_key)| {
        state
            .get(storage_key)
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(move |attributes| {
                attributes
                    .iter()
                    .map(move |(key, value)| (category.to_string(), key.clone(), value.clone()))
            })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::character::attribute_event::{SOURCE_KIND_AI_SYNC, SOURCE_KIND_USER};
    use chrono::Utc;
    use serde_json::json;

    fn profile(base: Value, current_state: Value) -> protagonist_profile::Model {
        let now = Utc::now().naive_utc();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: "林岚".to_string(),
            gender: None,
            age: None,
            base_attributes: base,
            current_state,
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter: 4,
            created_at: now,
            updated_at: now,
        }
    }

    fn event(id: i64, chapter: i32, source: &str, value: &str) -> attribute_event::Model {
        attribute_event::Model {
            id,
            profile_id: 1,
            chapter_number: chapter,
            source_kind: source.to_string(),
            timestamp: Utc::now().naive_utc(),
            category: "social".to_string(),
            key: "沈默".to_string(),
            operation: "modify".to_string(),
            change_description: "关系变化".to_string(),
            old_value: Value::Null,
            new_value: json!(value),
            cause: "正文事实".to_string(),
            evidence: "证据".to_string(),
        }
    }

    #[test]
    fn user_event_owns_the_field_from_its_chapter_without_erasing_earlier_ai_history() {
        let profile = profile(
            json!({
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {"沈默": "陌生人"}
            }),
            json!({
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {"沈默": "旧友"}
            }),
        );
        let events = vec![
            event(1, 2, SOURCE_KIND_AI_SYNC, "同伴"),
            event(2, 3, SOURCE_KIND_USER, "旧友"),
            event(3, 4, SOURCE_KIND_AI_SYNC, "敌人"),
        ];
        let ownership = character_attribute_ownership_from_records(&profile, &events, &[], true);

        assert!(!ownership.is_protected(2, "social", "沈默"));
        assert!(ownership.is_protected(3, "social", "沈默"));
        assert_eq!(
            replay_profile_state_from_records(&profile, &events, 4, &ownership).unwrap()["social_attributes"]
                ["沈默"],
            json!("旧友")
        );
    }

    #[test]
    fn manual_baseline_is_protected_from_the_first_chapter() {
        let profile = profile(
            json!({
                "explicit_attributes": {"occupation": "记者"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
            json!({
                "explicit_attributes": {"occupation": "记者"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        );
        let ownership = character_attribute_ownership_from_records(&profile, &[], &[], false);
        assert!(ownership.is_protected(1, "explicit", "occupation"));
    }
}
