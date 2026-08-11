use super::super::state::attribute_value_in_state;
use super::super::types::{
    CharacterAttributeDeletionRequestInput, CharacterDeletionMarkUpsertInput,
};
use super::updates::reset_pending_deletion_marks;
use super::upsert::upsert_deletion_mark;
use crate::character::{
    CharacterDbError, require_non_empty, require_profile_for_update, validate_category,
    validate_chapter_number, validate_timeline_source_kind,
};
use a4n_core::models::character::{attribute_event::SOURCE_KIND_USER, protagonist_deletion_mark};
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn request_attribute_deletion_in<C>(
    db: &C,
    input: CharacterAttributeDeletionRequestInput,
) -> Result<protagonist_deletion_mark::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    request_attribute_deletion_from_source_in(db, input, SOURCE_KIND_USER).await
}

pub(in crate::character) async fn request_attribute_deletion_from_source_in<C>(
    db: &C,
    input: CharacterAttributeDeletionRequestInput,
    source_kind: &str,
) -> Result<protagonist_deletion_mark::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let reason = require_non_empty(&input.reason, "删除原因")?;
    let evidence = require_non_empty(&input.evidence, "删除证据")?;
    let source_kind = validate_timeline_source_kind(source_kind)?;
    validate_category(&category)?;

    let profile = require_profile_for_update(db, input.profile_id).await?;
    if attribute_value_in_state(&profile.current_state, &category, &key).is_none() {
        return Err(CharacterDbError::NotFound(format!(
            "角色属性不存在: {category}.{key}"
        )));
    }

    upsert_deletion_mark(
        db,
        CharacterDeletionMarkUpsertInput {
            profile_id: input.profile_id,
            source_kind,
            category,
            key,
            chapter_number: input.chapter_number,
            reason,
            evidence,
        },
    )
    .await
    .map_err(CharacterDbError::Database)
}

pub async fn request_attribute_deletion<C>(
    db: &C,
    input: CharacterAttributeDeletionRequestInput,
) -> Result<protagonist_deletion_mark::Model, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let mark = request_attribute_deletion_in(&tx, input).await?;
    tx.commit().await?;
    Ok(mark)
}

pub async fn reset_deletion_marks_in<C>(
    db: &C,
    profile_id: i64,
    category: &str,
    key: &str,
) -> Result<u64, CharacterDbError>
where
    C: ConnectionTrait,
{
    let category = category.trim();
    let key = key.trim();
    validate_category(category)?;
    if key.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "属性 key 不能为空".to_string(),
        ));
    }

    reset_pending_deletion_marks(db, profile_id, category, key)
        .await
        .map_err(CharacterDbError::Database)
}
