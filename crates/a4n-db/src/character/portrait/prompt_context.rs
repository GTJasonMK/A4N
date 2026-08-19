use super::{
    CharacterPortraitRecordError, PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
};
use a4n_core::models::{
    character::{character_portrait, protagonist_profile},
    project::novel_blueprint,
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Map, Value};

pub async fn portrait_prompt_profile_rows<C>(
    db: &C,
    project_id: i64,
    profile_id: i64,
) -> Result<PortraitPromptProfileRows, CharacterPortraitRecordError>
where
    C: ConnectionTrait,
{
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(CharacterPortraitRecordError::Database)?
        .ok_or_else(|| {
            CharacterPortraitRecordError::NotFound(format!("角色档案不存在: {profile_id}"))
        })?;
    if profile.project_id != project_id {
        return Err(CharacterPortraitRecordError::InvalidArgument(format!(
            "角色档案 {profile_id} 不属于项目 {project_id}"
        )));
    }
    let project_context = portrait_project_context(db, project_id)
        .await
        .map_err(CharacterPortraitRecordError::Database)?;
    Ok(PortraitPromptProfileRows {
        profile,
        project_context,
    })
}

pub async fn portrait_named_prompt_profile_rows<C>(
    db: &C,
    project_id: i64,
    character_name: &str,
) -> Result<PortraitNamedPromptProfileRows, CharacterPortraitRecordError>
where
    C: ConnectionTrait,
{
    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(character_name))
        .one(db)
        .await
        .map_err(CharacterPortraitRecordError::Database)?
        .ok_or_else(|| {
            CharacterPortraitRecordError::NotFound(format!(
                "项目 {project_id} 中未找到角色「{character_name}」"
            ))
        })?;

    let existing_portrait_count = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile.id))
        .count(db)
        .await
        .map_err(CharacterPortraitRecordError::Database)?;
    let project_context = portrait_project_context(db, project_id)
        .await
        .map_err(CharacterPortraitRecordError::Database)?;

    Ok(PortraitNamedPromptProfileRows {
        profile,
        existing_portrait_count,
        project_context,
    })
}

async fn portrait_project_context<C>(db: &C, project_id: i64) -> Result<Option<Value>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(blueprint) = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let mut object = Map::new();
    insert_optional_text(&mut object, "title", Some(blueprint.title.as_str()));
    insert_optional_text(&mut object, "genre", Some(blueprint.genre.as_str()));
    insert_optional_text(&mut object, "style", blueprint.style.as_deref());
    insert_optional_text(&mut object, "tone", blueprint.tone.as_deref());
    insert_optional_text(
        &mut object,
        "target_audience",
        blueprint.target_audience.as_deref(),
    );
    insert_optional_text(
        &mut object,
        "one_sentence_summary",
        blueprint.one_sentence_summary.as_deref(),
    );
    if let Some(world_setting) = blueprint.world_setting.as_ref() {
        insert_meaningful_json(&mut object, "world_setting", world_setting);
    }
    if object.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(object)))
    }
}

fn insert_optional_text(object: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(text) = value.map(str::trim).filter(|text| !text.is_empty()) {
        object.insert(key.to_string(), Value::String(text.to_string()));
    }
}

fn insert_meaningful_json(object: &mut Map<String, Value>, key: &str, value: &Value) {
    if is_meaningful_json(value) {
        object.insert(key.to_string(), value.clone());
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}
