use super::super::{state::attribute_value_in_state, types::CharacterAttributeMutationResult};
use super::recording::{
    default_attribute_change_description, record_attribute_event_with_profile_in,
};
use crate::character::profile::require_profile_for_update;
use crate::character::{
    CharacterDbError, require_non_empty, validate_category, validate_chapter_number,
};
use a4n_db_models::models::character::{
    attribute_event::{self, SOURCE_KIND_USER},
    protagonist_profile,
};
use a4n_domain::character::CharacterAttributeValueChangeInput;
use sea_orm::{ConnectionTrait, TransactionTrait};
use serde_json::Value;

fn normalize_attribute_value_change_input(
    mut input: CharacterAttributeValueChangeInput,
) -> Result<CharacterAttributeValueChangeInput, CharacterDbError> {
    validate_chapter_number(input.chapter_number)?;
    input.category = input.category.trim().to_string();
    input.key = require_non_empty(&input.key, "属性 key")?;
    input.cause = require_non_empty(&input.cause, "属性变更原因")?;
    input.evidence = require_non_empty(&input.evidence, "属性变更证据")?;
    validate_category(&input.category)?;
    Ok(input)
}

async fn add_profile_attribute_with_profile_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeValueChangeInput,
    source_kind: &str,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    if attribute_value_in_state(&profile.current_state, &input.category, &input.key).is_some() {
        return Err(CharacterDbError::InvalidState {
            current: format!("属性 {}.{} 已存在", input.category, input.key),
            operation: "新增角色属性".to_string(),
        });
    }

    let change_description = match input.change_description {
        Some(value) => require_non_empty(&value, "属性变更摘要")?,
        None => default_attribute_change_description("add", &input.category, &input.key),
    };

    record_attribute_event_with_profile_in(
        db,
        profile,
        super::super::types::CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind: source_kind.to_string(),
            category: input.category,
            key: input.key,
            operation: "add".to_string(),
            old_value: Value::Null,
            new_value: input.value,
            change_description,
            cause: input.cause,
            evidence: input.evidence,
        },
    )
    .await
}

pub(in crate::character) async fn add_profile_attribute_with_profile_from_source_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeValueChangeInput,
    source_kind: &str,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let input = normalize_attribute_value_change_input(input)?;
    add_profile_attribute_with_profile_in(db, profile, input, source_kind).await
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
    let input = normalize_attribute_value_change_input(input)?;
    let event =
        add_profile_attribute_with_profile_in(&tx, profile.clone(), input, SOURCE_KIND_USER)
            .await?;
    tx.commit().await?;
    Ok(CharacterAttributeMutationResult { profile, event })
}

pub(in crate::character::attributes) async fn modify_profile_attribute_with_profile_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeValueChangeInput,
    source_kind: &str,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let old_value = attribute_value_in_state(&profile.current_state, &input.category, &input.key)
        .ok_or_else(|| {
        CharacterDbError::NotFound(format!("角色属性不存在: {}.{}", input.category, input.key))
    })?;

    let change_description = match input.change_description {
        Some(value) => require_non_empty(&value, "属性变更摘要")?,
        None => default_attribute_change_description("modify", &input.category, &input.key),
    };

    record_attribute_event_with_profile_in(
        db,
        profile,
        super::super::types::CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind: source_kind.to_string(),
            category: input.category,
            key: input.key,
            operation: "modify".to_string(),
            old_value,
            new_value: input.value,
            change_description,
            cause: input.cause,
            evidence: input.evidence,
        },
    )
    .await
}

pub(in crate::character) async fn modify_profile_attribute_with_profile_from_source_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeValueChangeInput,
    source_kind: &str,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let input = normalize_attribute_value_change_input(input)?;
    modify_profile_attribute_with_profile_in(db, profile, input, source_kind).await
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
    let input = normalize_attribute_value_change_input(input)?;
    let event =
        modify_profile_attribute_with_profile_in(&tx, profile.clone(), input, SOURCE_KIND_USER)
            .await?;
    tx.commit().await?;
    Ok(CharacterAttributeMutationResult { profile, event })
}
