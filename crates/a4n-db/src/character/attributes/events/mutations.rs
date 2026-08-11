use super::super::{
    state::attribute_value_in_state,
    types::{CharacterAttributeMutationResult, CharacterAttributeValueChangeInput},
};
use super::recording::{default_attribute_change_description, record_attribute_event_in};
use crate::character::{
    CharacterDbError, require_non_empty, require_profile_for_update, validate_category,
    validate_chapter_number,
};
use a4n_core::models::character::attribute_event::{self, SOURCE_KIND_USER};
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::Value;

pub async fn add_profile_attribute_in<C>(
    db: &C,
    input: CharacterAttributeValueChangeInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    add_profile_attribute_from_source_in(db, input, SOURCE_KIND_USER).await
}

pub(in crate::character) async fn add_profile_attribute_from_source_in<C>(
    db: &C,
    input: CharacterAttributeValueChangeInput,
    source_kind: &str,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let cause = require_non_empty(&input.cause, "属性变更原因")?;
    let evidence = require_non_empty(&input.evidence, "属性变更证据")?;
    validate_category(&category)?;

    let profile = require_profile_for_update(db, input.profile_id).await?;
    if attribute_value_in_state(&profile.current_state, &category, &key).is_some() {
        return Err(CharacterDbError::InvalidState {
            current: format!("属性 {category}.{key} 已存在"),
            operation: "新增角色属性".to_string(),
        });
    }

    let change_description = match input.change_description {
        Some(value) => require_non_empty(&value, "属性变更摘要")?,
        None => default_attribute_change_description("add", &category, &key),
    };

    record_attribute_event_in(
        db,
        super::super::types::CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind: source_kind.to_string(),
            category,
            key,
            operation: "add".to_string(),
            old_value: Value::Null,
            new_value: input.value,
            change_description,
            cause,
            evidence,
        },
    )
    .await
}

pub async fn add_profile_attribute<C>(
    db: &C,
    input: CharacterAttributeValueChangeInput,
) -> Result<CharacterAttributeMutationResult, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let profile_id = input.profile_id;
    let tx = db.begin().await?;
    let profile = require_profile_for_update(&tx, profile_id).await?;
    let event = add_profile_attribute_in(&tx, input).await?;
    tx.commit().await?;
    Ok(CharacterAttributeMutationResult { profile, event })
}

pub async fn modify_profile_attribute_in<C>(
    db: &C,
    input: CharacterAttributeValueChangeInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    modify_profile_attribute_from_source_in(db, input, SOURCE_KIND_USER).await
}

pub(in crate::character) async fn modify_profile_attribute_from_source_in<C>(
    db: &C,
    input: CharacterAttributeValueChangeInput,
    source_kind: &str,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let cause = require_non_empty(&input.cause, "属性变更原因")?;
    let evidence = require_non_empty(&input.evidence, "属性变更证据")?;
    validate_category(&category)?;

    let profile = require_profile_for_update(db, input.profile_id).await?;
    let old_value = attribute_value_in_state(&profile.current_state, &category, &key)
        .ok_or_else(|| CharacterDbError::NotFound(format!("角色属性不存在: {category}.{key}")))?;

    let change_description = match input.change_description {
        Some(value) => require_non_empty(&value, "属性变更摘要")?,
        None => default_attribute_change_description("modify", &category, &key),
    };

    record_attribute_event_in(
        db,
        super::super::types::CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind: source_kind.to_string(),
            category,
            key,
            operation: "modify".to_string(),
            old_value,
            new_value: input.value,
            change_description,
            cause,
            evidence,
        },
    )
    .await
}

pub async fn modify_profile_attribute<C>(
    db: &C,
    input: CharacterAttributeValueChangeInput,
) -> Result<CharacterAttributeMutationResult, CharacterDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let profile_id = input.profile_id;
    let tx = db.begin().await?;
    let profile = require_profile_for_update(&tx, profile_id).await?;
    let event = modify_profile_attribute_in(&tx, input).await?;
    tx.commit().await?;
    Ok(CharacterAttributeMutationResult { profile, event })
}
