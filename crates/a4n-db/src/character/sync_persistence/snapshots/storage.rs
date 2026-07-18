use std::collections::BTreeMap;

use sea_orm::ConnectionTrait;
use serde_json::{Value, json};

use super::super::{CharacterSyncPersistenceInput, CharacterSyncStoredSnapshotsRecord};
use super::projection::{
    character_sync_attribute_events_at_chapter, character_sync_count_items_at_chapter,
    character_sync_items_at_chapter,
};
use crate::character::{
    CharacterDbError, CharacterSnapshotReferenceUpsertInput, CharacterStateIndexUpsertInput,
    apply_attribute_to_state, attribute_events_by_profile_chapter_range,
    character_snapshot_content_hash, upsert_snapshot_reference, upsert_state_index,
};

pub(in crate::character::sync_persistence) async fn store_character_sync_snapshots_in<C>(
    db: &C,
    input: &CharacterSyncPersistenceInput,
    implicit_reviews_applied: usize,
) -> Result<CharacterSyncStoredSnapshotsRecord, CharacterDbError>
where
    C: ConnectionTrait,
{
    let events = attribute_events_by_profile_chapter_range(
        db,
        input.profile_id,
        input.start_chapter,
        input.target_chapter,
    )
    .await?;

    let mut attributes = input.baseline_state.clone();
    let mut event_index = 0usize;
    let mut target_hash = None;
    let mut target_attributes = None;
    let chapter_state_by_number = input
        .chapter_states
        .iter()
        .map(|state| (state.chapter_number, state))
        .collect::<BTreeMap<_, _>>();
    let mut state_indexes = Vec::with_capacity(input.selected_chapter_numbers.len());

    for chapter_number in &input.selected_chapter_numbers {
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
        let chapter_state = chapter_state_by_number.get(chapter_number).ok_or_else(|| {
            CharacterDbError::Internal(format!("第 {chapter_number} 章缺少状态索引摘要"))
        })?;
        let snapshot_data = json!({
            "profile_id": input.profile_id,
            "project_id": input.project_id,
            "name": input.profile_name,
            "synced_chapter": *chapter_number,
            "sync_range": {
                "from": input.start_chapter,
                "to": input.target_chapter,
            },
            "attributes": attributes.clone(),
            "state_summary": chapter_state.state_summary.clone(),
            "current_location": chapter_state.current_location.clone(),
            "current_goal": chapter_state.current_goal.clone(),
            "emotional_state": chapter_state.emotional_state.clone(),
            "changes": character_sync_attribute_events_at_chapter(&events, *chapter_number),
            "changes_in_chapter": changes_in_chapter,
            "behaviors": character_sync_items_at_chapter(&input.behaviors, *chapter_number)?,
            "behaviors_in_chapter": character_sync_count_items_at_chapter(
                &input.behaviors,
                *chapter_number,
            )?,
            "classification_suggestions": character_sync_items_at_chapter(
                &input.classification_suggestions,
                *chapter_number,
            )?,
            "classification_suggestions_applied": character_sync_count_items_at_chapter(
                &input.classification_suggestions,
                *chapter_number,
            )?,
            "deletion_candidates": character_sync_items_at_chapter(
                &input.deletion_candidates,
                *chapter_number,
            )?,
            "implicit_review_decisions": if *chapter_number == input.target_chapter {
                input.implicit_review_decisions.clone()
            } else {
                Value::Array(vec![])
            },
            "implicit_reviews_applied": if *chapter_number == input.target_chapter {
                implicit_reviews_applied
            } else {
                0
            },
        });
        let snapshot_hash = character_snapshot_content_hash(&snapshot_data)?;
        upsert_snapshot_reference(
            db,
            CharacterSnapshotReferenceUpsertInput {
                profile_id: input.profile_id,
                chapter_number: *chapter_number,
                snapshot_hash: snapshot_hash.clone(),
                data: snapshot_data,
            },
        )
        .await?;
        let state_index = upsert_state_index(
            db,
            CharacterStateIndexUpsertInput {
                profile_id: input.profile_id,
                chapter_number: *chapter_number,
                state_summary: chapter_state.state_summary.clone(),
                key_attributes: attributes.clone(),
                emotional_state: chapter_state.emotional_state.clone(),
                current_location: chapter_state.current_location.clone(),
                current_goal: chapter_state.current_goal.clone(),
            },
        )
        .await?;
        state_indexes.push(state_index);

        if *chapter_number == input.target_chapter {
            target_hash = Some(snapshot_hash);
            target_attributes = Some(attributes.clone());
        }
    }

    Ok(CharacterSyncStoredSnapshotsRecord {
        target_hash: target_hash.ok_or_else(|| {
            CharacterDbError::Internal(format!(
                "未能生成目标章节快照: 第 {} 章",
                input.target_chapter
            ))
        })?,
        target_attributes: target_attributes.ok_or_else(|| {
            CharacterDbError::Internal(format!(
                "未能折叠目标章节状态: 第 {} 章",
                input.target_chapter
            ))
        })?,
        state_indexes,
    })
}
