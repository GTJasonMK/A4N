use a4n_core::models::project::{project_source_snapshot, source_snapshot_chapter};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;

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

pub async fn create_source_snapshot_in_record<C>(
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

    for chapter in data.chapters {
        let snapshot_chapter = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title),
            content: Set(chapter.content.clone()),
            summary: Set(chapter.summary),
            analysis_data_json: Set(chapter.analysis_data.clone()),
            content_hash: Set(hash_text(&chapter.content)),
            analysis_hash: Set(chapter.analysis_data.as_ref().map(hash_json)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectMutationError::Database)?;
        crate::text_anchor::insert_content_anchor_records(
            db,
            crate::text_anchor::TextAnchorContentInsertInput {
                project_id: root_project_id,
                source_chapter_id: Some(snapshot_chapter.source_chapter_id),
                source_snapshot_id: Some(snapshot.id),
                source_snapshot_chapter_id: Some(snapshot_chapter.id),
                source_id: snapshot_chapter.id,
                chapter_number: snapshot_chapter.chapter_number,
                content: &snapshot_chapter.content,
                source_version_hash: &snapshot_chapter.content_hash,
            },
        )
        .await?;
    }

    freeze_outlines(db, snapshot.id, root_project_id, scope).await?;
    freeze_profiles(db, snapshot.id, root_project_id).await?;

    Ok(snapshot)
}
