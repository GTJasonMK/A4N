use a4n_db_models::models::project::{project_source_snapshot, source_snapshot_chapter};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde_json::Value;
use std::collections::HashMap;

use super::data::build_source_snapshot_data;
use crate::project::{
    ProjectMutationError,
    source_snapshot::{
        DerivedChapterScope,
        hashing::{hash_json, hash_text},
        outlines::freeze_outlines,
        profiles::freeze_profiles,
    },
};

struct PendingSnapshotAnchor {
    source_chapter_id: i64,
    source_snapshot_chapter_id: i64,
    source_id: i64,
    chapter_number: i32,
    content: String,
    source_version_hash: String,
}

pub(crate) async fn create_source_snapshot_in_record<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
    now: NaiveDateTime,
) -> Result<project_source_snapshot::Model, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let data = build_source_snapshot_data(db, root_project_id, scope).await?;

    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root_project_id),
        source_version_hash: Set(data.source_version_hash),
        blueprint_version_id: Set(data.latest_blueprint_version_id),
        chapter_scope_json: Set(data.chapter_scope_json.clone()),
        chapter_hashes_json: Set(Value::Array(data.chapter_hashes_json.clone())),
        profile_hashes_json: Set(data.profile_hashes_json.clone()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectMutationError::Database)?;

    let snapshot_chapter_rows = data
        .chapters
        .iter()
        .map(|chapter| source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title.clone()),
            content: Set(chapter.content.clone()),
            summary: Set(chapter.summary.clone()),
            analysis_data_json: Set(chapter.analysis_data.clone()),
            content_hash: Set(hash_text(&chapter.content)),
            analysis_hash: Set(chapter.analysis_data.as_ref().map(hash_json)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted_snapshot_chapters = if snapshot_chapter_rows.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::insert_many(snapshot_chapter_rows)
            .exec_with_returning_many(db)
            .await
            .map_err(ProjectMutationError::Database)?
    };
    if inserted_snapshot_chapters.len() != data.chapters.len() {
        return Err(ProjectMutationError::Internal(format!(
            "源快照章节批量写入返回行数不一致: inserted={}, expected={}",
            inserted_snapshot_chapters.len(),
            data.chapters.len()
        )));
    }
    let mut snapshot_chapters_by_source_id =
        HashMap::with_capacity(inserted_snapshot_chapters.len());
    for snapshot_chapter in inserted_snapshot_chapters {
        if snapshot_chapters_by_source_id
            .insert(snapshot_chapter.source_chapter_id, snapshot_chapter)
            .is_some()
        {
            return Err(ProjectMutationError::Internal(
                "源快照章节 source_chapter_id 重复".to_string(),
            ));
        }
    }

    let mut pending_anchors = Vec::with_capacity(data.chapters.len());
    for chapter in data.chapters {
        let snapshot_chapter = snapshot_chapters_by_source_id
            .remove(&chapter.id)
            .ok_or_else(|| {
                ProjectMutationError::Internal(format!(
                    "源快照章节批量写入后缺少 source_chapter_id={}",
                    chapter.id
                ))
            })?;
        pending_anchors.push(PendingSnapshotAnchor {
            source_chapter_id: snapshot_chapter.source_chapter_id,
            source_snapshot_chapter_id: snapshot_chapter.id,
            source_id: snapshot_chapter.id,
            chapter_number: snapshot_chapter.chapter_number,
            content: snapshot_chapter.content,
            source_version_hash: snapshot_chapter.content_hash,
        });
    }

    if !pending_anchors.is_empty() {
        let inputs = pending_anchors
            .iter()
            .map(|pending| crate::text_anchor::TextAnchorContentInsertInput {
                project_id: root_project_id,
                source_chapter_id: Some(pending.source_chapter_id),
                source_snapshot_id: Some(snapshot.id),
                source_snapshot_chapter_id: Some(pending.source_snapshot_chapter_id),
                source_id: pending.source_id,
                chapter_number: pending.chapter_number,
                content: &pending.content,
                source_version_hash: &pending.source_version_hash,
            })
            .collect::<Vec<_>>();
        crate::text_anchor::insert_content_anchor_records_batch(db, &inputs).await?;
    }

    freeze_outlines(db, snapshot.id, root_project_id, scope).await?;
    freeze_profiles(db, snapshot.id, root_project_id, data.profiles).await?;

    Ok(snapshot)
}
