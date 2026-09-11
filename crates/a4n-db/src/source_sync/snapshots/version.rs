use super::super::SourceSyncDbError;
use crate::project::DerivedChapterScope;
use a4n_db_models::models::{
    asset::object_profile, chapter::chapter, character::protagonist_profile,
    project::blueprint_version, scene::scene_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(in crate::source_sync) async fn calculate_source_sync_version_hash<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<String, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .column(chapter::Column::AnalysisData)
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::ChapterNumber.gte(scope.start_chapter))
        .filter(chapter::Column::ChapterNumber.lte(scope.end_chapter))
        .order_by_asc(chapter::Column::ChapterNumber)
        .into_tuple::<(i64, i32, String, Option<Value>)>()
        .all(db)
        .await?;
    if chapters.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(
            "源文本快照范围内没有章节".to_string(),
        ));
    }
    let expected_numbers = (scope.start_chapter..=scope.end_chapter).collect::<Vec<_>>();
    let actual_numbers = chapters
        .iter()
        .map(|(_, chapter_number, _, _)| *chapter_number)
        .collect::<Vec<_>>();
    if actual_numbers != expected_numbers {
        let actual_number_set = actual_numbers.iter().copied().collect::<HashSet<_>>();
        let missing = expected_numbers
            .into_iter()
            .filter(|number| !actual_number_set.contains(number))
            .collect::<Vec<_>>();
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "源文本快照范围内缺少章节: {:?}",
            missing
        )));
    }

    let chapter_hashes_json = chapters
        .iter()
        .map(|(id, chapter_number, content, analysis_data)| {
            json!({
                "source_chapter_id": id,
                "chapter_number": chapter_number,
                "content_hash": hash_text(content),
                "analysis_hash": analysis_data.as_ref().map(hash_json),
            })
        })
        .collect::<Vec<_>>();
    let profile_hashes_json = source_sync_profile_hashes_json(db, root_project_id).await?;
    let latest_blueprint_version_id = blueprint_version::Entity::find()
        .select_only()
        .column(blueprint_version::Column::Id)
        .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .order_by_desc(blueprint_version::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await?;
    let chapter_scope_json = source_sync_chapter_scope_json(scope);
    Ok(hash_json(&json!({
        "root_project_id": root_project_id,
        "chapter_scope": chapter_scope_json,
        "chapter_hashes": chapter_hashes_json,
        "profile_hashes": profile_hashes_json,
        "blueprint_version_id": latest_blueprint_version_id,
    })))
}

fn source_sync_chapter_scope_json(scope: DerivedChapterScope) -> Value {
    json!({
        "start": scope.start_chapter,
        "end": scope.end_chapter,
    })
}

async fn source_sync_profile_hashes_json<C>(
    db: &C,
    root_project_id: i64,
) -> Result<Value, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let mut profiles = Vec::new();
    for profile in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await?
    {
        profiles.push(json!({
            "kind": "character",
            "id": profile.id,
            "hash": hash_json(&to_json(&profile)?),
        }));
    }
    for profile in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await?
    {
        profiles.push(json!({
            "kind": "object",
            "id": profile.id,
            "hash": hash_json(&to_json(&profile)?),
        }));
    }
    for profile in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await?
    {
        profiles.push(json!({
            "kind": "scene",
            "id": profile.id,
            "hash": hash_json(&to_json(&profile)?),
        }));
    }
    Ok(Value::Array(profiles))
}

fn to_json<T>(value: &T) -> Result<Value, SourceSyncDbError>
where
    T: Serialize,
{
    serde_json::to_value(value)
        .map_err(|err| SourceSyncDbError::Internal(format!("源快照序列化失败: {err}")))
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}
