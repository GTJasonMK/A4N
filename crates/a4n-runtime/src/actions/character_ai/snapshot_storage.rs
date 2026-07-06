use crate::actions::character_implicit_review::ImplicitUpdateDecisionSummary;
use crate::state::AppState;
use a4n_core::models::character::{attribute_event, protagonist_profile};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use super::json_utils::required_i32;
use super::state_projection::apply_attribute_to_state;
use super::sync_response::ChapterStateDraft;

#[derive(Debug, Clone)]
pub(super) struct StoredChapterSnapshots {
    pub(super) target_hash: String,
    pub(super) target_attributes: Value,
    pub(super) states: Vec<StoredChapterState>,
}

#[derive(Debug, Clone)]
pub(super) struct StoredChapterState {
    pub(super) chapter_number: i32,
    pub(super) state_summary: String,
    pub(super) current_location: Option<String>,
    pub(super) current_goal: Option<String>,
    pub(super) emotional_state: Option<String>,
    pub(super) attributes: Value,
}

pub(super) struct StoreChapterSnapshotsInput<'a, C>
where
    C: ConnectionTrait,
{
    pub(super) state: &'a AppState,
    pub(super) db: &'a C,
    pub(super) profile: &'a protagonist_profile::Model,
    pub(super) project_id: i64,
    pub(super) profile_name: &'a str,
    pub(super) chapter_numbers: &'a [i32],
    pub(super) start_chapter: i32,
    pub(super) target_chapter: i32,
    pub(super) baseline_state: &'a Value,
    pub(super) behaviors: &'a [Value],
    pub(super) classification_suggestions: &'a [Value],
    pub(super) deletion_candidates: &'a [Value],
    pub(super) chapter_states: &'a [ChapterStateDraft],
    pub(super) implicit_review_decisions: &'a [ImplicitUpdateDecisionSummary],
    pub(super) implicit_reviews_applied: usize,
}

pub(super) async fn store_chapter_snapshots<C>(
    input: StoreChapterSnapshotsInput<'_, C>,
) -> Result<StoredChapterSnapshots, String>
where
    C: ConnectionTrait,
{
    let StoreChapterSnapshotsInput {
        state,
        db,
        profile,
        project_id,
        profile_name,
        chapter_numbers,
        start_chapter,
        target_chapter,
        baseline_state,
        behaviors,
        classification_suggestions,
        deletion_candidates,
        chapter_states,
        implicit_review_decisions,
        implicit_reviews_applied,
    } = input;
    let events = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.eq(profile.id))
        .filter(attribute_event::Column::ChapterNumber.gte(start_chapter))
        .filter(attribute_event::Column::ChapterNumber.lte(target_chapter))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    let mut attributes = baseline_state.clone();
    let mut event_index = 0usize;
    let mut target_hash = None;
    let mut target_attributes = None;
    let chapter_state_by_number = chapter_states
        .iter()
        .map(|state| (state.chapter_number, state))
        .collect::<BTreeMap<_, _>>();
    let mut states = Vec::with_capacity(chapter_numbers.len());
    for chapter_number in chapter_numbers {
        while let Some(event) = events.get(event_index) {
            if event.chapter_number > *chapter_number {
                break;
            }
            apply_attribute_to_state(
                &mut attributes,
                &event.category,
                &event.key,
                &event.operation,
                event.new_value.clone(),
            )?;
            event_index += 1;
        }
        let changes_in_chapter = events
            .iter()
            .filter(|event| event.chapter_number == *chapter_number)
            .count();
        let changes = attribute_events_at_chapter(&events, *chapter_number);
        let behaviors_in_chapter = count_items_at_chapter(behaviors, *chapter_number)?;
        let chapter_state = chapter_state_by_number
            .get(chapter_number)
            .ok_or_else(|| format!("第 {chapter_number} 章缺少状态索引摘要"))?;
        let snapshot_data = json!({
            "profile_id": profile.id,
            "project_id": project_id,
            "name": profile_name,
            "synced_chapter": *chapter_number,
            "sync_range": {
                "from": start_chapter,
                "to": target_chapter,
            },
            "attributes": attributes.clone(),
            "state_summary": chapter_state.state_summary.clone(),
            "current_location": chapter_state.current_location.clone(),
            "current_goal": chapter_state.current_goal.clone(),
            "emotional_state": chapter_state.emotional_state.clone(),
            "changes": changes,
            "changes_in_chapter": changes_in_chapter,
            "behaviors": items_at_chapter(behaviors, *chapter_number)?,
            "behaviors_in_chapter": behaviors_in_chapter,
            "classification_suggestions": items_at_chapter(
                classification_suggestions,
                *chapter_number,
            )?,
            "classification_suggestions_applied": count_items_at_chapter(
                classification_suggestions,
                *chapter_number,
            )?,
            "deletion_candidates": items_at_chapter(deletion_candidates, *chapter_number)?,
            "implicit_review_decisions": if *chapter_number == target_chapter {
                serde_json::to_value(implicit_review_decisions).map_err(|e| e.to_string())?
            } else {
                Value::Array(vec![])
            },
            "implicit_reviews_applied": if *chapter_number == target_chapter {
                implicit_reviews_applied
            } else {
                0
            },
        });
        let snapshot = state
            .snapshot_store
            .store_snapshot_in(
                db,
                "character_profile",
                profile.id,
                *chapter_number as i64,
                snapshot_data,
            )
            .await
            .map_err(|e| e.to_string())?;
        states.push(StoredChapterState {
            chapter_number: *chapter_number,
            state_summary: chapter_state.state_summary.clone(),
            current_location: chapter_state.current_location.clone(),
            current_goal: chapter_state.current_goal.clone(),
            emotional_state: chapter_state.emotional_state.clone(),
            attributes: attributes.clone(),
        });
        if *chapter_number == target_chapter {
            target_hash = Some(snapshot.content_hash);
            target_attributes = Some(attributes.clone());
        }
    }

    Ok(StoredChapterSnapshots {
        target_hash: target_hash
            .ok_or_else(|| format!("未能生成目标章节快照: 第 {target_chapter} 章"))?,
        target_attributes: target_attributes
            .ok_or_else(|| format!("未能折叠目标章节状态: 第 {target_chapter} 章"))?,
        states,
    })
}

fn items_at_chapter(items: &[Value], chapter_number: i32) -> Result<Value, String> {
    let mut matched = Vec::new();
    for item in items {
        if required_i32(item, "chapter_number")? == chapter_number {
            matched.push(item.clone());
        }
    }
    Ok(Value::Array(matched))
}

pub(super) fn attribute_events_at_chapter(
    events: &[attribute_event::Model],
    chapter_number: i32,
) -> Value {
    Value::Array(
        events
            .iter()
            .filter(|event| event.chapter_number == chapter_number)
            .map(|event| {
                json!({
                    "id": event.id,
                    "chapter_number": event.chapter_number,
                    "timestamp": event.timestamp.to_string(),
                    "category": event.category.clone(),
                    "key": event.key.clone(),
                    "operation": event.operation.clone(),
                    "change_description": event.change_description.clone(),
                    "old_value": event.old_value.clone(),
                    "new_value": event.new_value.clone(),
                    "cause": event.cause.clone(),
                    "evidence": event.evidence.clone(),
                })
            })
            .collect(),
    )
}

fn count_items_at_chapter(items: &[Value], chapter_number: i32) -> Result<usize, String> {
    let mut count = 0;
    for item in items {
        if required_i32(item, "chapter_number")? == chapter_number {
            count += 1;
        }
    }
    Ok(count)
}
