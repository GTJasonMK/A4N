use super::fields::{
    optional_blueprint_snapshot_json, optional_blueprint_snapshot_strength,
    optional_blueprint_snapshot_string, required_blueprint_snapshot_string,
};
use super::*;

pub(super) async fn restore_blueprint_character_snapshot_record<C>(
    db: &C,
    project_id: i64,
    snapshot: Option<&Value>,
    now: NaiveDateTime,
) -> Result<(), BlueprintRestoreError>
where
    C: ConnectionTrait,
{
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.is_null() {
        return Ok(());
    }
    let drafts = parse_blueprint_character_drafts(snapshot)?;
    sync_blueprint_characters(db, project_id, drafts, now)
        .await
        .map_err(|source| BlueprintRestoreError::CharacterSync { source })?;
    Ok(())
}

pub(super) async fn restore_blueprint_relationship_snapshot_record<C>(
    db: &C,
    project_id: i64,
    snapshot: Option<&Value>,
    now: NaiveDateTime,
) -> Result<(), BlueprintRestoreError>
where
    C: ConnectionTrait,
{
    let Some(snapshot) = snapshot else {
        return Ok(());
    };
    if snapshot.is_null() {
        return Ok(());
    }
    let drafts = parse_blueprint_relationship_drafts(snapshot)?;
    sync_blueprint_relationships(db, project_id, drafts, now)
        .await
        .map_err(|source| BlueprintRestoreError::RelationshipSync { source })?;
    Ok(())
}

fn parse_blueprint_character_drafts(
    value: &Value,
) -> Result<Vec<BlueprintCharacterDraft>, BlueprintRestoreError> {
    let array = value
        .as_array()
        .ok_or_else(|| BlueprintRestoreError::invalid_snapshot("蓝图版本 characters 必须是数组"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let label = format!("蓝图版本 characters[{index}]");
            Ok(BlueprintCharacterDraft {
                name: required_blueprint_snapshot_string(item, "name", &label)?,
                character_type: required_blueprint_snapshot_string(item, "character_type", &label)?,
                gender: optional_blueprint_snapshot_string(item, "gender", &label)?,
                age: optional_blueprint_snapshot_string(item, "age", &label)?,
                appearance: optional_blueprint_snapshot_string(item, "appearance", &label)?,
                personality: optional_blueprint_snapshot_string(item, "personality", &label)?,
                background: optional_blueprint_snapshot_string(item, "background", &label)?,
                motivation: optional_blueprint_snapshot_string(item, "motivation", &label)?,
                arc_summary: optional_blueprint_snapshot_string(item, "arc_summary", &label)?,
                relationship_to_protagonist: optional_blueprint_snapshot_string(
                    item,
                    "relationship_to_protagonist",
                    &label,
                )?,
                arc_stages: optional_blueprint_snapshot_json(item, "arc_stages"),
            })
        })
        .collect()
}

pub(super) fn parse_blueprint_relationship_drafts(
    value: &Value,
) -> Result<Vec<BlueprintRelationshipDraft>, BlueprintRestoreError> {
    let array = value.as_array().ok_or_else(|| {
        BlueprintRestoreError::invalid_snapshot("蓝图版本 relationships 必须是数组")
    })?;
    array
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let label = format!("蓝图版本 relationships[{index}]");
            Ok(BlueprintRelationshipDraft {
                relationship_type: required_blueprint_snapshot_string(
                    item,
                    "relationship_type",
                    &label,
                )?,
                character_a: required_blueprint_snapshot_string(item, "character_a", &label)?,
                character_b: required_blueprint_snapshot_string(item, "character_b", &label)?,
                description: optional_blueprint_snapshot_string(item, "description", &label)?,
                strength: optional_blueprint_snapshot_strength(item, "strength", &label)?,
                evolution: optional_blueprint_snapshot_json(item, "evolution"),
            })
        })
        .collect()
}
