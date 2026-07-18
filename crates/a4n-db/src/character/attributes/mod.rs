mod deletion_marks;
mod events;
mod implicit_review;
mod state;
mod types;

pub use deletion_marks::*;
pub use events::*;
pub use implicit_review::*;
pub(super) use state::category_storage_key;
pub use state::*;
pub use types::*;

use super::{
    CharacterDbError, require_non_empty, require_profile, validate_category,
    validate_chapter_number,
};
use a4n_core::models::character::attribute_event;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::Value;
use types::CharacterAttributeDeletionRecordInput;

async fn execute_attribute_deletion_in<C>(
    db: &C,
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
    validate_category(&category)?;
    if input.mark_count <= 0 {
        return Err(CharacterDbError::InvalidArgument(
            "删除标记次数必须大于 0".to_string(),
        ));
    }

    let profile = require_profile(db, input.profile_id).await?;
    let old_value = state::attribute_value_in_state(&profile.current_state, &category, &key)
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色属性不存在: {category}.{key}")))?;
    let change_description =
        events::default_attribute_change_description("delete", &category, &key);
    let event = events::record_attribute_event_in(
        db,
        CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
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

pub async fn execute_ready_attribute_deletion_in<C>(
    db: &C,
    input: CharacterAttributeDeletionExecutionInput,
) -> Result<(attribute_event::Model, i32), CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let event_cause = require_non_empty(&input.event_cause, "删除原因")?;
    let evidence = require_non_empty(&input.evidence, "删除证据")?;
    validate_category(&category)?;

    let mark =
        deletion_marks::pending_deletion_mark_for_attribute(db, input.profile_id, &category, &key)
            .await?
            .ok_or_else(|| {
                CharacterDbError::NotFound(format!("删除执行缺少未执行标记: {category}.{key}"))
            })?;

    if mark.consecutive_count < DELETION_THRESHOLD {
        return Err(CharacterDbError::InvalidState {
            current: format!(
                "删除标记连续次数 {}/{}",
                mark.consecutive_count, DELETION_THRESHOLD
            ),
            operation: "执行角色属性删除".to_string(),
        });
    }

    let count = mark.consecutive_count;
    let event = execute_attribute_deletion_in(
        db,
        CharacterAttributeDeletionRecordInput {
            profile_id: input.profile_id,
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

pub async fn execute_ready_attribute_deletion<C>(
    db: &C,
    input: CharacterAttributeDeletionExecutionInput,
) -> Result<CharacterAttributeDeletionExecutionResult, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let profile_id = input.profile_id;
    let tx = db.begin().await?;
    let profile = require_profile(&tx, profile_id).await?;
    let (event, consecutive_count) = execute_ready_attribute_deletion_in(&tx, input).await?;
    tx.commit().await?;
    Ok(CharacterAttributeDeletionExecutionResult {
        profile,
        event,
        consecutive_count,
    })
}

#[cfg(test)]
mod tests;
