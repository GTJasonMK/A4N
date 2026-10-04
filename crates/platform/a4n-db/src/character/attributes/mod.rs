mod deletion_marks;
mod events;
mod implicit_review;
mod state;
mod types;

#[cfg(test)]
pub(in crate::character::attributes) use deletion_marks::request_attribute_deletion_in;
pub(in crate::character) use deletion_marks::request_attribute_deletion_with_profile_in;
pub use deletion_marks::{
    deletion_mark_records_by_profile, deletion_marks_by_profile, request_attribute_deletion,
    request_attribute_deletion_record, reset_deletion_marks_in, rewind_ai_deletion_marks_after,
    rewind_deletion_marks_after,
};
pub(in crate::character) use events::attribute_events_by_profile_chapter_range;
#[cfg(test)]
pub(in crate::character::attributes) use events::record_attribute_event_in;
pub use events::{
    add_profile_attribute, attribute_events_by_ids, attribute_history, modify_profile_attribute,
};
pub(in crate::character) use events::{
    add_profile_attribute_with_profile_from_source_in,
    modify_profile_attribute_with_profile_from_source_in,
};
pub use implicit_review::apply_implicit_review_decision_record;
pub(in crate::character) use implicit_review::apply_implicit_review_decision_record_with_profile_from_source_in;
pub(in crate::character) use state::apply_attribute_to_state;
pub(in crate::character) use state::attribute_value_in_state;
pub use types::{
    AppliedCharacterImplicitDecisionAction, AppliedCharacterImplicitDecisionRecord,
    CharacterAttributeDeletionExecutionResult, CharacterAttributeEventHistoryFilter,
    CharacterAttributeMutationResult, CharacterDeletionMarkRewindSummary,
};
pub(in crate::character::attributes) use types::{
    CharacterAttributeEventInsertInput, CharacterDeletionMarkExecutedUpdateInput,
};

use super::profile::require_profile_for_update;
use super::{
    CharacterDbError, require_non_empty, validate_category, validate_chapter_number,
    validate_timeline_source_kind,
};
use a4n_db_models::models::character::attribute_event::SOURCE_KIND_USER;
use a4n_db_models::models::character::{attribute_event, protagonist_profile};
use a4n_domain::character::{
    CHARACTER_DELETION_THRESHOLD, CharacterAttributeDeletionExecutionInput,
};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::Value;
use types::CharacterAttributeDeletionRecordInput;

async fn execute_attribute_deletion_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeDeletionRecordInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let event_cause = require_non_empty(&input.event_cause, "删除原因")?;
    let evidence = require_non_empty(&input.evidence, "删除证据")?;
    let source_kind = validate_timeline_source_kind(&input.source_kind)?;
    validate_category(&category)?;
    if input.mark_count <= 0 {
        return Err(CharacterDbError::InvalidArgument(
            "删除标记次数必须大于 0".to_string(),
        ));
    }

    let old_value = state::attribute_value_in_state(&profile.current_state, &category, &key)
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色属性不存在: {category}.{key}")))?;
    let change_description =
        events::default_attribute_change_description("delete", &category, &key);
    let event = events::record_attribute_event_with_profile_in(
        db,
        profile,
        CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind,
            category: category.clone(),
            key: key.clone(),
            operation: "delete".to_string(),
            old_value,
            new_value: Value::Null,
            change_description,
            cause: event_cause,
            evidence,
        },
    )
    .await?;

    let updated = deletion_marks::mark_deletion_executed(
        db,
        CharacterDeletionMarkExecutedUpdateInput {
            profile_id: input.profile_id,
            category: category.clone(),
            key: key.clone(),
            consecutive_count: input.mark_count,
        },
    )
    .await?;
    if updated == 0 {
        return Err(CharacterDbError::NotFound(format!(
            "删除执行缺少未执行标记: {category}.{key}"
        )));
    }

    Ok(event)
}

#[cfg(test)]
async fn execute_ready_attribute_deletion_in<C>(
    db: &C,
    input: CharacterAttributeDeletionExecutionInput,
) -> Result<(attribute_event::Model, i32), CharacterDbError>
where
    C: ConnectionTrait,
{
    execute_ready_attribute_deletion_with_profile_in(db, input, SOURCE_KIND_USER, None).await
}

async fn execute_ready_attribute_deletion_with_profile_in<C>(
    db: &C,
    input: CharacterAttributeDeletionExecutionInput,
    source_kind: &str,
    profile: Option<protagonist_profile::Model>,
) -> Result<(attribute_event::Model, i32), CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let event_cause = require_non_empty(&input.event_cause, "删除原因")?;
    let evidence = require_non_empty(&input.evidence, "删除证据")?;
    let source_kind = validate_timeline_source_kind(source_kind)?;
    validate_category(&category)?;

    let mark =
        deletion_marks::pending_deletion_mark_for_attribute(db, input.profile_id, &category, &key)
            .await?
            .ok_or_else(|| {
                CharacterDbError::NotFound(format!("删除执行缺少未执行标记: {category}.{key}"))
            })?;

    if mark.consecutive_count < CHARACTER_DELETION_THRESHOLD {
        return Err(CharacterDbError::InvalidState {
            current: format!(
                "删除标记连续次数 {}/{}",
                mark.consecutive_count, CHARACTER_DELETION_THRESHOLD
            ),
            operation: "执行角色属性删除".to_string(),
        });
    }

    let count = mark.consecutive_count;
    let profile = match profile {
        Some(profile) => profile,
        None => require_profile_for_update(db, input.profile_id).await?,
    };
    let event = execute_attribute_deletion_in(
        db,
        profile,
        CharacterAttributeDeletionRecordInput {
            profile_id: input.profile_id,
            source_kind,
            category,
            key,
            mark_count: count,
            event_cause,
            evidence,
            chapter_number: input.chapter_number,
        },
    )
    .await?;
    Ok((event, count))
}

pub(in crate::character) async fn execute_ready_attribute_deletion_with_profile_from_source_in<C>(
    db: &C,
    input: CharacterAttributeDeletionExecutionInput,
    source_kind: &str,
    profile: protagonist_profile::Model,
) -> Result<(attribute_event::Model, i32), CharacterDbError>
where
    C: ConnectionTrait,
{
    execute_ready_attribute_deletion_with_profile_in(db, input, source_kind, Some(profile)).await
}

pub async fn execute_ready_attribute_deletion<C>(
    db: &C,
    input: CharacterAttributeDeletionExecutionInput,
) -> Result<CharacterAttributeDeletionExecutionResult, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let profile_id = input.profile_id;
    let tx = db.begin().await?;
    let profile = require_profile_for_update(&tx, profile_id).await?;
    let (event, consecutive_count) = execute_ready_attribute_deletion_with_profile_in(
        &tx,
        input,
        SOURCE_KIND_USER,
        Some(profile.clone()),
    )
    .await?;
    tx.commit().await?;
    Ok(CharacterAttributeDeletionExecutionResult {
        profile: crate::character::character_profile_record(profile),
        event: crate::character::character_attribute_event_record(event),
        consecutive_count,
    })
}

#[cfg(test)]
mod tests;
