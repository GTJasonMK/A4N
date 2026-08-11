use super::super::{
    deletion_marks::reset_pending_deletion_marks, state::apply_attribute_to_state,
    types::CharacterAttributeEventInsertInput,
};
use super::validation::validate_operation;
use crate::character::{
    CharacterDbError, CharacterProfileCurrentStateUpdate, profile_current_state_fields,
    require_non_empty, require_profile_for_update, update_profile_current_state, validate_category,
    validate_chapter_number, validate_timeline_source_kind,
};
use a4n_core::models::character::attribute_event;
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

pub async fn insert_attribute_event<C>(
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

pub async fn record_attribute_event_in<C>(
    db: &C,
    input: CharacterAttributeEventInsertInput,
) -> Result<attribute_event::Model, CharacterDbError>
where
    C: ConnectionTrait,
{
    validate_chapter_number(input.chapter_number)?;
    let category = input.category.trim().to_string();
    let key = require_non_empty(&input.key, "属性 key")?;
    let operation = input.operation.trim().to_string();
    let source_kind = validate_timeline_source_kind(&input.source_kind)?;
    let change_description = require_non_empty(&input.change_description, "属性变更摘要")?;
    let cause = require_non_empty(&input.cause, "属性变更原因")?;
    let evidence = require_non_empty(&input.evidence, "属性变更证据")?;
    validate_category(&category)?;
    validate_operation(&operation)?;

    let profile = require_profile_for_update(db, input.profile_id).await?;
    let event = insert_attribute_event(
        db,
        CharacterAttributeEventInsertInput {
            profile_id: input.profile_id,
            chapter_number: input.chapter_number,
            source_kind,
            category: category.clone(),
            key: key.clone(),
            operation: operation.clone(),
            old_value: input.old_value,
            new_value: input.new_value,
            change_description,
            cause,
            evidence,
        },
    )
    .await?;

    let mut next_state = profile.current_state.clone();
    apply_attribute_to_state(
        &mut next_state,
        &category,
        &key,
        &operation,
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

    if matches!(operation.as_str(), "add" | "modify") {
        reset_pending_deletion_marks(db, input.profile_id, &category, &key).await?;
    }

    Ok(event)
}
