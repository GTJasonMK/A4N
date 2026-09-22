pub(super) mod store;
use super::CharacterDbError;
use a4n_db_models::models::character::snapshot_reference;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CharacterSnapshotReferenceUpsertInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub data: Value,
}

pub(super) fn character_snapshot_content_hash(data: &Value) -> Result<String, serde_json::Error> {
    let json = serde_json::to_string(data)?;
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn snapshot_attributes_from_data(
    data: &Value,
    chapter_number: i32,
) -> Result<Value, CharacterDbError> {
    let attributes = data
        .get("attributes")
        .ok_or_else(|| CharacterDbError::InvalidState {
            current: format!("第 {chapter_number} 章角色快照缺少 attributes"),
            operation: "读取角色快照属性".to_string(),
        })?;
    let object = attributes
        .as_object()
        .ok_or_else(|| CharacterDbError::InvalidState {
            current: format!("第 {chapter_number} 章角色快照 attributes 不是 JSON 对象"),
            operation: "读取角色快照属性".to_string(),
        })?;

    for key in [
        "explicit_attributes",
        "implicit_attributes",
        "social_attributes",
    ] {
        if !object.get(key).is_some_and(Value::is_object) {
            return Err(CharacterDbError::InvalidState {
                current: format!(
                    "第 {chapter_number} 章角色快照 attributes.{key} 缺少或不是 JSON 对象"
                ),
                operation: "读取角色快照属性".to_string(),
            });
        }
    }

    Ok(attributes.clone())
}

pub async fn snapshot_references_by_profile<C>(
    db: &C,
    profile_id: i64,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<Vec<snapshot_reference::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id));
    if let Some(start_chapter) = start_chapter {
        query = query.filter(snapshot_reference::Column::ChapterNumber.gte(start_chapter));
    }
    if let Some(end_chapter) = end_chapter {
        query = query.filter(snapshot_reference::Column::ChapterNumber.lte(end_chapter));
    }
    query
        .order_by_asc(snapshot_reference::Column::ChapterNumber)
        .all(db)
        .await
}

pub async fn snapshot_reference_by_chapter<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<Option<snapshot_reference::Model>, DbErr>
where
    C: ConnectionTrait,
{
    snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::ProfileId.eq(profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
}

pub(super) async fn upsert_snapshot_reference<C>(
    db: &C,
    input: CharacterSnapshotReferenceUpsertInput,
) -> Result<snapshot_reference::Model, DbErr>
where
    C: ConnectionTrait,
{
    let existing_id = snapshot_reference::Entity::find()
        .select_only()
        .column(snapshot_reference::Column::Id)
        .filter(snapshot_reference::Column::ProfileId.eq(input.profile_id))
        .filter(snapshot_reference::Column::ChapterNumber.eq(input.chapter_number))
        .into_tuple::<i64>()
        .one(db)
        .await?;

    if let Some(id) = existing_id {
        return snapshot_reference::ActiveModel {
            id: Set(id),
            snapshot_hash: Set(input.snapshot_hash),
            reference_count: Set(1),
            data: Set(input.data),
            ..Default::default()
        }
        .update(db)
        .await;
    }

    snapshot_reference::ActiveModel {
        profile_id: Set(input.profile_id),
        snapshot_hash: Set(input.snapshot_hash),
        chapter_number: Set(input.chapter_number),
        reference_count: Set(1),
        data: Set(input.data),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub(super) async fn snapshot_reference_by_hash<C>(
    db: &C,
    content_hash: &str,
) -> Result<Option<snapshot_reference::Model>, DbErr>
where
    C: ConnectionTrait,
{
    snapshot_reference::Entity::find()
        .filter(snapshot_reference::Column::SnapshotHash.eq(content_hash))
        .order_by_desc(snapshot_reference::Column::CreatedAt)
        .one(db)
        .await
}

#[cfg(test)]
mod tests;
