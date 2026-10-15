use super::super::state::attribute_value_in_state;
use super::super::types::CharacterDeletionMarkUpsertInput;
use super::updates::reset_pending_deletion_mark_if_current;
use super::upsert::upsert_deletion_mark;
use crate::character::profile::require_profile_for_update;
use crate::character::{
    CharacterDbError, require_non_empty, validate_category, validate_chapter_number,
    validate_timeline_source_kind,
};
use a4n_db_models::models::character::{
    attribute_event::SOURCE_KIND_USER, protagonist_deletion_mark, protagonist_profile,
};
use a4n_domain::character::CharacterAttributeDeletionRequestInput;
use a4n_domain::character::CharacterDeletionMarkRecord;
use chrono::NaiveDateTime;
use sea_orm::{ConnectionTrait, TransactionTrait};

pub(in crate::character::attributes) async fn request_attribute_deletion_in<C>(
    db: &C,
    input: CharacterAttributeDeletionRequestInput,
) -> Result<protagonist_deletion_mark::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let validated = validate_attribute_deletion_request(&input, SOURCE_KIND_USER)?;
    let profile = require_profile_for_update(db, input.profile_id).await?;
    request_attribute_deletion_with_profile_validated(db, &profile, &input, validated).await
}

pub(in crate::character) async fn request_attribute_deletion_with_profile_in<C>(
    db: &C,
    profile: &protagonist_profile::Model,
    input: CharacterAttributeDeletionRequestInput,
    source_kind: &str,
) -> Result<protagonist_deletion_mark::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let validated = validate_attribute_deletion_request(&input, source_kind)?;
    request_attribute_deletion_with_profile_validated(db, profile, &input, validated).await
}

struct ValidatedAttributeDeletionRequest {
    category: String,
    key: String,
    reason: String,
    evidence: String,
    source_kind: String,
}

fn validate_attribute_deletion_request(
    input: &CharacterAttributeDeletionRequestInput,
    source_kind: &str,
) -> Result<ValidatedAttributeDeletionRequest, CharacterDbError> {
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let reason = require_non_empty(&input.reason, "删除原因")?;
    let evidence = require_non_empty(&input.evidence, "删除证据")?;
    let source_kind = validate_timeline_source_kind(source_kind)?;
    validate_category(&category)?;
    Ok(ValidatedAttributeDeletionRequest {
        category,
        key,
        reason,
        evidence,
        source_kind,
    })
}

async fn request_attribute_deletion_with_profile_validated<C>(
    db: &C,
    profile: &protagonist_profile::Model,
    input: &CharacterAttributeDeletionRequestInput,
    validated: ValidatedAttributeDeletionRequest,
) -> Result<protagonist_deletion_mark::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let ValidatedAttributeDeletionRequest {
        category,
        key,
        reason,
        evidence,
        source_kind,
    } = validated;

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

pub async fn request_attribute_deletion_record<C>(
    db: &C,
    input: CharacterAttributeDeletionRequestInput,
) -> Result<CharacterDeletionMarkRecord, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    request_attribute_deletion(db, input)
        .await
        .map(crate::character::character_deletion_mark_record)
}

pub async fn reset_deletion_marks_in<C>(
    db: &C,
    profile_id: i64,
    category: &str,
    key: &str,
    expected_mark_id: i64,
    expected_mark_updated_at: NaiveDateTime,
) -> Result<u64, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let category = category.trim();
    let key = key.trim();
    validate_category(category)?;
    if key.is_empty() {
        return Err(CharacterDbError::InvalidArgument(
            "属性 key 不能为空".to_string(),
        ));
    }

    let tx = db.begin().await?;
    require_profile_for_update(&tx, profile_id).await?;
    let deleted = reset_pending_deletion_mark_if_current(
        &tx,
        profile_id,
        category,
        key,
        expected_mark_id,
        expected_mark_updated_at,
    )
    .await
    .map_err(CharacterDbError::Database)?;
    if deleted == 0 {
        return Err(CharacterDbError::InvalidState {
            current: "删除标记已变化".to_string(),
            operation: "重置角色属性删除标记".to_string(),
        });
    }
    tx.commit().await?;
    Ok(deleted)
}
