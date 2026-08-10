use super::super::{
    CharacterDbError, SummaryField, apply_attribute_to_state, attribute_value_in_state,
    profile_base_attributes_are_ai_generated, summary_field_has_untracked_value,
    summary_field_value,
};
use a4n_core::models::character::{
    attribute_event::{self, SOURCE_KIND_AI_SYNC},
    protagonist_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;
use std::collections::HashSet;

/// 从角色基础档案重放属性事件，得到指定章节截止时的状态投影。
///
/// 快照和状态索引是可重建的派生数据，不能在保留用户事件后继续作为唯一状态来源。
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

    // 读取完整事件历史来判断字段归属，再只把截止章节前的事件重放。
    // 不能用 profile.current_state 判断历史归属：它通常是最新投影，重同步旧章节时
    // 可能包含未来章节的 AI 结果。
    let all_events = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile.id))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id)
        .all(db)
        .await?;
    let base_attributes_are_ai_generated =
        profile_base_attributes_are_ai_generated(db, profile).await?;
    replay_profile_state_from_events(
        profile,
        &all_events,
        chapter_number,
        base_attributes_are_ai_generated,
    )
}

fn replay_profile_state_from_events(
    profile: &protagonist_profile::Model,
    all_events: &[attribute_event::Model],
    chapter_number: i32,
    base_attributes_are_ai_generated: bool,
) -> Result<Value, CharacterDbError> {
    let mut ai_blocked_attributes =
        initial_ai_blocked_attributes(profile, all_events, base_attributes_are_ai_generated);
    let mut state = profile.base_attributes.clone();
    restore_untracked_current_state_attributes(&mut state, profile, all_events)?;
    for event in all_events
        .iter()
        .filter(|event| event.chapter_number <= chapter_number)
    {
        replay_attribute_event_with_ownership(&mut state, event, &mut ai_blocked_attributes)?;
    }
    restore_untracked_summary_fields(&mut state, profile)?;
    Ok(state)
}

#[cfg(test)]
fn replay_attribute_event(
    state: &mut Value,
    event: &attribute_event::Model,
) -> Result<(), CharacterDbError> {
    validate_attribute_event(event)?;
    apply_replayed_attribute_event(state, event)
}

fn replay_attribute_event_with_ownership(
    state: &mut Value,
    event: &attribute_event::Model,
    ai_blocked_attributes: &mut HashSet<(String, String)>,
) -> Result<(), CharacterDbError> {
    validate_attribute_event(event)?;
    let attribute = (event.category.clone(), event.key.clone());
    if event.source_kind == SOURCE_KIND_AI_SYNC {
        if ai_blocked_attributes.contains(&attribute) {
            return Ok(());
        }
    } else {
        // 用户/历史事件只从它发生的章节起拥有该属性；更早的 AI 事件仍然
        // 是历史事实，不能为了保护后来的用户编辑而从历史投影中抹掉。
        ai_blocked_attributes.insert(attribute);
    }
    apply_replayed_attribute_event(state, event)
}

fn apply_replayed_attribute_event(
    state: &mut Value,
    event: &attribute_event::Model,
) -> Result<(), CharacterDbError> {
    apply_attribute_to_state(
        state,
        &event.category,
        &event.key,
        &event.operation,
        event.new_value.clone(),
    )
    .map_err(|error| CharacterDbError::InvalidState {
        current: format!("属性事件 {} 无法应用: {error}", event.id),
        operation: "重放角色属性事件".to_string(),
    })
}

fn validate_attribute_event(event: &attribute_event::Model) -> Result<(), CharacterDbError> {
    if !matches!(event.category.as_str(), "explicit" | "implicit" | "social") {
        return Err(CharacterDbError::InvalidState {
            current: format!("属性事件 {} 的 category 无效: {}", event.id, event.category),
            operation: "重放角色属性事件".to_string(),
        });
    }
    if !matches!(event.operation.as_str(), "add" | "modify" | "delete") {
        return Err(CharacterDbError::InvalidState {
            current: format!(
                "属性事件 {} 的 operation 无效: {}",
                event.id, event.operation
            ),
            operation: "重放角色属性事件".to_string(),
        });
    }
    if !super::super::is_valid_attribute_event_source_kind(&event.source_kind) {
        return Err(CharacterDbError::InvalidState {
            current: format!(
                "属性事件 {} 的 source_kind 无效: {}",
                event.id, event.source_kind
            ),
            operation: "重放角色属性事件".to_string(),
        });
    }
    Ok(())
}

fn initial_ai_blocked_attributes(
    profile: &protagonist_profile::Model,
    events: &[attribute_event::Model],
    base_attributes_are_ai_generated: bool,
) -> HashSet<(String, String)> {
    let mut blocked = HashSet::new();
    if !base_attributes_are_ai_generated {
        blocked.extend(
            attribute_entries(&profile.base_attributes)
                .into_iter()
                .map(|(category, key, _)| (category, key)),
        );
    }

    let event_attributes = events
        .iter()
        .map(|event| (event.category.clone(), event.key.clone()))
        .collect::<HashSet<_>>();
    blocked.extend(
        attribute_entries(&profile.current_state)
            .into_iter()
            .filter(|(category, key, _)| {
                !event_attributes.contains(&(category.clone(), key.clone()))
                    && attribute_value_in_state(&profile.base_attributes, category, key).is_none()
            })
            .map(|(category, key, _)| (category, key)),
    );

    blocked.extend(summary_fields().into_iter().filter_map(|field| {
        summary_field_has_untracked_value(profile, field).then(|| {
            let (category, key) = field.category_key();
            (category.to_string(), key.to_string())
        })
    }));
    blocked
}

fn restore_untracked_current_state_attributes(
    state: &mut Value,
    profile: &protagonist_profile::Model,
    events: &[attribute_event::Model],
) -> Result<(), CharacterDbError> {
    let event_attributes = events
        .iter()
        .map(|event| (event.category.as_str(), event.key.as_str()))
        .collect::<HashSet<_>>();
    for (category, key, value) in attribute_entries(&profile.current_state) {
        if event_attributes.contains(&(category.as_str(), key.as_str()))
            || attribute_value_in_state(&profile.base_attributes, &category, &key).is_some()
        {
            continue;
        }
        apply_attribute_to_state(state, &category, &key, "modify", value)?;
    }
    Ok(())
}

fn restore_untracked_summary_fields(
    state: &mut Value,
    profile: &protagonist_profile::Model,
) -> Result<(), CharacterDbError> {
    for field in summary_fields() {
        if !summary_field_has_untracked_value(profile, field) {
            continue;
        }
        let value = summary_field_value(profile, field).map(ToOwned::to_owned);
        let Some(value) = value else {
            continue;
        };
        let (category, key) = field.category_key();
        apply_attribute_to_state(state, category, key, "modify", Value::String(value))?;
    }
    Ok(())
}

fn summary_fields() -> [SummaryField; 5] {
    [
        SummaryField::Gender,
        SummaryField::Age,
        SummaryField::Appearance,
        SummaryField::Background,
        SummaryField::Personality,
    ]
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
    use super::{replay_attribute_event, replay_profile_state_from_events};
    use a4n_core::models::character::{
        attribute_event::{self, SOURCE_KIND_AI_SYNC, SOURCE_KIND_USER},
        protagonist_profile,
    };
    use chrono::Utc;
    use serde_json::json;

    fn event(
        id: i64,
        chapter_number: i32,
        source_kind: &str,
        operation: &str,
        key: &str,
        new_value: serde_json::Value,
    ) -> attribute_event::Model {
        attribute_event::Model {
            id,
            profile_id: 1,
            chapter_number,
            source_kind: source_kind.to_string(),
            timestamp: Utc::now().naive_utc(),
            category: "explicit".to_string(),
            key: key.to_string(),
            operation: operation.to_string(),
            change_description: "测试".to_string(),
            old_value: json!(null),
            new_value,
            cause: "测试".to_string(),
            evidence: "测试".to_string(),
        }
    }

    fn profile(
        base: serde_json::Value,
        current_state: serde_json::Value,
    ) -> protagonist_profile::Model {
        let now = Utc::now().naive_utc();
        protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: "测试角色".to_string(),
            gender: current_state
                .get("explicit_attributes")
                .and_then(|value| value.get("gender"))
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
            age: None,
            base_attributes: base,
            current_state,
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn replay_rejects_unknown_operation_instead_of_treating_it_as_modify() {
        let mut state = json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        });
        let error = replay_attribute_event(
            &mut state,
            &event(1, 1, SOURCE_KIND_USER, "replace", "位置", json!("v1")),
        )
        .expect_err("unknown operation must fail explicitly");
        assert!(error.to_string().contains("operation 无效"));
    }

    #[test]
    fn replay_applies_delete_to_canonical_state() {
        let mut state = json!({
            "explicit_attributes": {"位置": "城内"},
            "implicit_attributes": {},
            "social_attributes": {}
        });
        replay_attribute_event(
            &mut state,
            &event(2, 1, SOURCE_KIND_USER, "delete", "位置", json!(null)),
        )
        .expect("delete should apply");
        assert!(state["explicit_attributes"].get("位置").is_none());
    }

    #[test]
    fn replay_applies_ai_before_user_then_preserves_user_owner() {
        let base = json!({
            "explicit_attributes": {"gender": "初始"},
            "implicit_attributes": {},
            "social_attributes": {}
        });
        let profile = profile(
            base.clone(),
            json!({
                "explicit_attributes": {"gender": "用户最终"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        );
        let events = vec![
            event(
                1,
                2,
                SOURCE_KIND_AI_SYNC,
                "modify",
                "gender",
                json!("AI历史"),
            ),
            event(
                2,
                3,
                SOURCE_KIND_USER,
                "modify",
                "gender",
                json!("用户确认"),
            ),
            event(
                3,
                4,
                SOURCE_KIND_AI_SYNC,
                "modify",
                "gender",
                json!("AI冲突"),
            ),
        ];

        let before_user = replay_profile_state_from_events(&profile, &events, 2, true).unwrap();
        assert_eq!(
            before_user["explicit_attributes"]["gender"],
            json!("AI历史")
        );
        let after_user = replay_profile_state_from_events(&profile, &events, 4, true).unwrap();
        assert_eq!(
            after_user["explicit_attributes"]["gender"],
            json!("用户确认")
        );
    }

    #[test]
    fn replay_does_not_leak_future_ai_projection_into_an_earlier_chapter() {
        let base = json!({
            "explicit_attributes": {"gender": "初始"},
            "implicit_attributes": {},
            "social_attributes": {}
        });
        let profile = profile(
            base.clone(),
            json!({
                "explicit_attributes": {"gender": "未来 AI"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        );
        let events = vec![event(
            1,
            5,
            SOURCE_KIND_AI_SYNC,
            "modify",
            "gender",
            json!("未来 AI"),
        )];

        let state = replay_profile_state_from_events(&profile, &events, 2, true).unwrap();
        assert_eq!(state["explicit_attributes"]["gender"], json!("初始"));
    }

    #[test]
    fn replay_restores_untracked_top_level_user_field_without_events() {
        let profile = protagonist_profile::Model {
            gender: Some("用户直接设定".to_string()),
            ..profile(
                json!({
                    "explicit_attributes": {},
                    "implicit_attributes": {},
                    "social_attributes": {}
                }),
                json!({
                    "explicit_attributes": {},
                    "implicit_attributes": {},
                    "social_attributes": {}
                }),
            )
        };
        let state = replay_profile_state_from_events(&profile, &[], 3, true).unwrap();
        assert_eq!(
            state["explicit_attributes"]["gender"],
            json!("用户直接设定")
        );
    }

    #[test]
    fn replay_preserves_manual_profile_baseline_from_ai_events() {
        let profile = profile(
            json!({
                "explicit_attributes": {"weapon": "用户设定的剑"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
            json!({
                "explicit_attributes": {"weapon": "AI改写"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        );
        let events = vec![event(
            1,
            2,
            SOURCE_KIND_AI_SYNC,
            "modify",
            "weapon",
            json!("AI改写"),
        )];

        let state = replay_profile_state_from_events(&profile, &events, 2, false).unwrap();
        assert_eq!(
            state["explicit_attributes"]["weapon"],
            json!("用户设定的剑")
        );
    }

    #[test]
    fn replay_preserves_user_owner_for_non_summary_attribute() {
        let profile = profile(
            json!({
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
            json!({
                "explicit_attributes": {"weapon": "用户确认"},
                "implicit_attributes": {},
                "social_attributes": {}
            }),
        );
        let events = vec![
            event(1, 2, SOURCE_KIND_AI_SYNC, "add", "weapon", json!("AI初稿")),
            event(
                2,
                3,
                SOURCE_KIND_USER,
                "modify",
                "weapon",
                json!("用户确认"),
            ),
            event(
                3,
                4,
                SOURCE_KIND_AI_SYNC,
                "modify",
                "weapon",
                json!("AI冲突"),
            ),
        ];

        let state = replay_profile_state_from_events(&profile, &events, 4, true).unwrap();
        assert_eq!(state["explicit_attributes"]["weapon"], json!("用户确认"));
    }
}
