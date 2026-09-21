use super::super::{
    deletion_marks::reset_pending_deletion_marks, state::apply_attribute_to_state,
    types::CharacterAttributeEventInsertInput,
};
use super::validation::validate_operation;
#[cfg(test)]
use crate::character::profile::require_profile_for_update;
use crate::character::profile::{
    CharacterProfileCurrentStateUpdate, profile_current_state_fields, update_profile_current_state,
};
use crate::character::{
    CharacterDbError, require_non_empty, validate_category, validate_chapter_number,
    validate_timeline_source_kind,
};
use a4n_db_models::models::character::{attribute_event, protagonist_profile};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set};

pub(in crate::character::attributes) fn default_attribute_change_description(
    operation: &str,
    category: &str,
    key: &str,
) -> String {
    let action = match operation {
        "add" => "新增",
        "modify" => "修改",
        "delete" => "删除",
        _ => "变更",
    };
    format!("{action}{category}属性: {key}")
}

async fn insert_attribute_event<C>(
    db: &C,
    input: CharacterAttributeEventInsertInput,
) -> Result<attribute_event::Model, DbErr>
where
    C: ConnectionTrait,
{
    attribute_event::ActiveModel {
        profile_id: Set(input.profile_id),
        chapter_number: Set(input.chapter_number),
        source_kind: Set(input.source_kind),
        timestamp: Set(chrono::Utc::now().naive_utc()),
        category: Set(input.category),
        key: Set(input.key),
        operation: Set(input.operation),
        change_description: Set(input.change_description),
        old_value: Set(input.old_value),
        new_value: Set(input.new_value),
        cause: Set(input.cause),
        evidence: Set(input.evidence),
        ..Default::default()
    }
    .insert(db)
    .await
}

#[cfg(test)]
pub(in crate::character::attributes) async fn record_attribute_event_in<C>(
    db: &C,
    input: CharacterAttributeEventInsertInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let input = normalize_attribute_event_input(input)?;
    let profile = require_profile_for_update(db, input.profile_id).await?;
    record_attribute_event_for_profile_in(db, profile, input).await
}

pub(in crate::character::attributes) async fn record_attribute_event_with_profile_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeEventInsertInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let input = normalize_attribute_event_input(input)?;
    record_attribute_event_for_profile_in(db, profile, input).await
}

fn normalize_attribute_event_input(
    mut input: CharacterAttributeEventInsertInput,
) -> Result<CharacterAttributeEventInsertInput, CharacterDbError> {
    validate_chapter_number(input.chapter_number)?;
    input.category = input.category.trim().to_string();
    input.key = require_non_empty(&input.key, "属性 key")?;
    input.operation = input.operation.trim().to_string();
    input.source_kind = validate_timeline_source_kind(&input.source_kind)?;
    input.change_description = require_non_empty(&input.change_description, "属性变更摘要")?;
    input.cause = require_non_empty(&input.cause, "属性变更原因")?;
    input.evidence = require_non_empty(&input.evidence, "属性变更证据")?;
    validate_category(&input.category)?;
    validate_operation(&input.operation)?;
    Ok(input)
}

async fn record_attribute_event_for_profile_in<C>(
    db: &C,
    profile: protagonist_profile::Model,
    input: CharacterAttributeEventInsertInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    let event = insert_attribute_event(
        db,
        CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind: input.source_kind,
            category: input.category.clone(),
            key: input.key.clone(),
            operation: input.operation.clone(),
            old_value: input.old_value,
            new_value: input.new_value,
            change_description: input.change_description,
            cause: input.cause,
            evidence: input.evidence,
        },
    )
    .await?;

    let mut next_state = profile.current_state.clone();
    apply_attribute_to_state(
        &mut next_state,
        &input.category,
        &input.key,
        &input.operation,
        event.new_value.clone(),
    )?;
    let fields = profile_current_state_fields(next_state);
    update_profile_current_state(
        db,
        profile,
        CharacterProfileCurrentStateUpdate {
            gender: fields.gender,
            age: fields.age,
            appearance: fields.appearance,
            background: fields.background,
            personality: fields.personality,
            current_state: fields.current_state,
            last_synced_chapter: None,
        },
    )
    .await?;

    if matches!(input.operation.as_str(), "add" | "modify") {
        reset_pending_deletion_marks(db, input.profile_id, &input.category, &input.key).await?;
    }

    Ok(event)
}
