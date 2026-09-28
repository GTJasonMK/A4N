use a4n_db_models::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, part_outline},
    character::protagonist_profile,
    project::{
        blueprint_version, novel_project, project_source_snapshot, source_snapshot_chapter,
        source_snapshot_outline, source_snapshot_profile, text_anchor,
    },
    scene::scene_profile,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde_json::{Value, json};

use super::hash_value;

pub(super) async fn freeze_live_source<C: ConnectionTrait>(
    db: &C,
    root: &novel_project::Model,
    chapter_id: i64,
    expected_hash: &str,
    now: sea_orm::prelude::DateTime,
) -> Result<
    (
        project_source_snapshot::Model,
        source_snapshot_chapter::Model,
    ),
    DbErr,
> {
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .filter(chapter::Column::ProjectId.eq(root.id))
        .one(db)
        .await?
        .ok_or_else(|| {
            DbErr::Custom(format!(
                "历史导演来源章节不存在: root={} chapter={chapter_id}",
                root.id
            ))
        })?;
    if a4n_domain::source_text_hash(&chapter.content) != expected_hash {
        return matching_frozen_source(db, root.id, chapter_id, expected_hash).await;
    }
    if chapter.content.trim().is_empty() {
        return Err(DbErr::Custom("历史导演章节正文为空".into()));
    }
    let mut profiles = Vec::<(&str, i64, Value)>::new();
    for row in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root.id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await?
    {
        profiles.push((
            "character",
            row.id,
            serde_json::to_value(row).map_err(json_error)?,
        ));
    }
    for row in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root.id))
        .order_by_asc(object_profile::Column::Id)
        .all(db)
        .await?
    {
        profiles.push((
            "object",
            row.id,
            serde_json::to_value(row).map_err(json_error)?,
        ));
    }
    for row in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root.id))
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await?
    {
        profiles.push((
            "scene",
            row.id,
            serde_json::to_value(row).map_err(json_error)?,
        ));
    }
    let profile_hashes = profiles
        .iter()
        .map(|(kind, id, payload)| {
            Ok(json!({"kind": kind, "id": id, "hash": hash_value(payload)?}))
        })
        .collect::<Result<Vec<_>, DbErr>>()?;
    let blueprint = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(root.id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .order_by_desc(blueprint_version::Column::Id)
        .one(db)
        .await?
        .map(|row| row.id);
    let scope = json!({"start": chapter.chapter_number, "end": chapter.chapter_number});
    let analysis_hash = chapter.analysis_data.as_ref().map(hash_value).transpose()?;
    let chapter_hashes = json!([{
        "source_chapter_id": chapter.id, "chapter_number": chapter.chapter_number,
        "content_hash": expected_hash, "analysis_hash": analysis_hash,
    }]);
    let hash = hash_value(&json!({
        "root_project_id": root.id, "chapter_scope": scope, "chapter_hashes": chapter_hashes,
        "profile_hashes": profile_hashes, "blueprint_version_id": blueprint,
    }))?;
    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root.id),
        source_version_hash: Set(hash),
        blueprint_version_id: Set(blueprint),
        chapter_scope_json: Set(scope),
        chapter_hashes_json: Set(chapter_hashes),
        profile_hashes_json: Set(json!(profile_hashes)),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let frozen = source_snapshot_chapter::ActiveModel {
        snapshot_id: Set(snapshot.id),
        root_project_id: Set(root.id),
        source_chapter_id: Set(chapter.id),
        chapter_number: Set(chapter.chapter_number),
        title: Set(chapter.title),
        content: Set(chapter.content),
        summary: Set(chapter.summary),
        analysis_data_json: Set(chapter.analysis_data),
        content_hash: Set(expected_hash.into()),
        analysis_hash: Set(analysis_hash),
        ..Default::default()
    }
    .insert(db)
    .await?;
    for (kind, id, payload) in profiles {
        source_snapshot_profile::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(root.id),
            profile_kind: Set(kind.into()),
            source_profile_id: Set(id),
            payload_hash: Set(hash_value(&payload)?),
            payload_json: Set(payload),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    for outline in part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(root.id))
        .filter(part_outline::Column::StartChapter.lte(frozen.chapter_number))
        .filter(part_outline::Column::EndChapter.gte(frozen.chapter_number))
        .all(db)
        .await?
    {
        insert_outline(
            db,
            snapshot.id,
            root.id,
            outline.id,
            "part",
            serde_json::to_value(outline).map_err(json_error)?,
        )
        .await?;
    }
    for outline in chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(root.id))
        .filter(chapter_outline::Column::ChapterNumber.eq(frozen.chapter_number))
        .all(db)
        .await?
    {
        insert_outline(
            db,
            snapshot.id,
            root.id,
            outline.id,
            "chapter",
            serde_json::to_value(outline).map_err(json_error)?,
        )
        .await?;
    }
    for span in a4n_domain::source_spans::source_spans_for_content(
        frozen.source_chapter_id,
        frozen.chapter_number,
        &frozen.content,
    ) {
        text_anchor::ActiveModel {
            project_id: Set(root.id),
            source_chapter_id: Set(Some(frozen.source_chapter_id)),
            source_snapshot_id: Set(Some(snapshot.id)),
            source_snapshot_chapter_id: Set(Some(frozen.id)),
            source_kind: Set("chapter_content".into()),
            source_id: Set(frozen.id),
            paragraph_index: Set(i32::try_from(span.sentence_index)
                .map_err(|error| DbErr::Custom(error.to_string()))?),
            start_offset: Set(
                i32::try_from(span.char_start).map_err(|error| DbErr::Custom(error.to_string()))?
            ),
            end_offset: Set(
                i32::try_from(span.char_end).map_err(|error| DbErr::Custom(error.to_string()))?
            ),
            quote_hash: Set(a4n_domain::source_text_hash(&span.text)),
            quote: Set(span.text),
            source_version_hash: Set(frozen.content_hash.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok((snapshot, frozen))
}

async fn insert_outline<C: ConnectionTrait>(
    db: &C,
    snapshot_id: i64,
    root_id: i64,
    id: i64,
    kind: &str,
    payload: Value,
) -> Result<(), DbErr> {
    source_snapshot_outline::ActiveModel {
        snapshot_id: Set(snapshot_id),
        root_project_id: Set(root_id),
        source_outline_id: Set(id),
        outline_kind: Set(kind.into()),
        payload_hash: Set(hash_value(&payload)?),
        payload_json: Set(payload),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

async fn matching_frozen_source<C: ConnectionTrait>(
    db: &C,
    root_id: i64,
    chapter_id: i64,
    hash: &str,
) -> Result<
    (
        project_source_snapshot::Model,
        source_snapshot_chapter::Model,
    ),
    DbErr,
> {
    let chapters = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(chapter_id))
        .filter(source_snapshot_chapter::Column::ContentHash.eq(hash))
        .order_by_asc(source_snapshot_chapter::Column::Id)
        .all(db)
        .await?;
    let mut selected: Option<(
        project_source_snapshot::Model,
        source_snapshot_chapter::Model,
    )> = None;
    for chapter in chapters {
        if a4n_domain::source_text_hash(&chapter.content) != hash {
            return Err(DbErr::Custom("历史正文快照哈希损坏".into()));
        }
        let snapshot = project_source_snapshot::Entity::find_by_id(chapter.snapshot_id)
            .one(db)
            .await?
            .ok_or_else(|| DbErr::Custom("历史章节的来源快照不存在".into()))?;
        if let Some((previous, previous_chapter)) = &selected {
            if previous.profile_hashes_json != snapshot.profile_hashes_json
                || previous.blueprint_version_id != snapshot.blueprint_version_id
                || previous_chapter.chapter_number != chapter.chapter_number
            {
                return Err(DbErr::Custom(format!(
                    "历史章节 {chapter_id} 存在多个不同档案版本，不能推测导演来源"
                )));
            }
        } else {
            selected = Some((snapshot, chapter));
        }
    }
    selected.ok_or_else(|| {
        DbErr::Custom(format!(
            "无法恢复历史导演正文: chapter={chapter_id} sha256={hash}"
        ))
    })
}

fn json_error(error: serde_json::Error) -> DbErr {
    DbErr::Custom(error.to_string())
}
