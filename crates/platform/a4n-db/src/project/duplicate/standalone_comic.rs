use a4n_db_models::models::{
    canvas::{canvas_text_asset, canvas_text_asset_version},
    chapter::chapter,
    project::{project_source_snapshot, source_snapshot_chapter, standalone_comic_text_source},
};
use a4n_domain::project::{ProjectRecord, ProjectSourceMode};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::collections::HashMap;

use super::{
    ProjectDuplicateError,
    shared::{ProjectCopyInsert, insert_project_copy_record},
};
use crate::project::{project_record_from_model, project_record_into_model};

pub async fn duplicate_standalone_comic_project_record(
    db: &DatabaseConnection,
    original: ProjectRecord,
    user_id: i64,
    new_title: &str,
) -> Result<ProjectRecord, ProjectDuplicateError> {
    if original.source_mode().map_err(|error| {
        ProjectDuplicateError::InvalidArgument(format!("项目 {}: {error}", original.id))
    })? != ProjectSourceMode::Standalone
    {
        return Err(ProjectDuplicateError::InvalidArgument(
            "复制独立漫画项目只能接收 standalone comic 项目".to_string(),
        ));
    }

    let original = project_record_into_model(original);
    let txn = db.begin().await?;
    let now = Utc::now().naive_utc();
    let project = insert_project_copy_record(
        &txn,
        &original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            project_kind: original.project_kind.clone(),
            root_project_id: None,
            source_snapshot_id: None,
            status: original.status.clone(),
            current_chapter: 0,
            total_chapters: None,
            raw_blueprint: None,
        },
        now,
    )
    .await?;
    let asset_ids = copy_text_assets(&txn, original.id, project.id, now).await?;
    copy_text_sources(&txn, original.id, project.id, &asset_ids, now).await?;
    txn.commit().await?;
    Ok(project_record_from_model(project))
}

async fn copy_text_assets<C: ConnectionTrait>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError> {
    let assets = canvas_text_asset::Entity::find()
        .filter(canvas_text_asset::Column::ProjectId.eq(source_project_id))
        .order_by_asc(canvas_text_asset::Column::Id)
        .all(db)
        .await?;
    let mut ids = HashMap::with_capacity(assets.len());
    for asset in assets {
        let copy = canvas_text_asset::ActiveModel {
            project_id: Set(target_project_id),
            title: Set(asset.title.clone()),
            text_kind: Set(asset.text_kind.clone()),
            content: Set(asset.content.clone()),
            content_hash: Set(asset.content_hash.clone()),
            current_version: Set(asset.current_version),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        ids.insert(asset.id, copy.id);
        let versions = canvas_text_asset_version::Entity::find()
            .filter(canvas_text_asset_version::Column::CanvasTextAssetId.eq(asset.id))
            .order_by_asc(canvas_text_asset_version::Column::Version)
            .all(db)
            .await?;
        for version in versions {
            canvas_text_asset_version::ActiveModel {
                canvas_text_asset_id: Set(copy.id),
                version: Set(version.version),
                content: Set(version.content),
                content_hash: Set(version.content_hash),
                created_at: Set(version.created_at),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
    }
    Ok(ids)
}

async fn copy_text_sources<C: ConnectionTrait>(
    db: &C,
    source_project_id: i64,
    target_project_id: i64,
    asset_ids: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), ProjectDuplicateError> {
    let sources = standalone_comic_text_source::Entity::find()
        .filter(standalone_comic_text_source::Column::ProjectId.eq(source_project_id))
        .order_by_asc(standalone_comic_text_source::Column::Id)
        .all(db)
        .await?;
    for source in sources {
        let chapter = chapter::Entity::find_by_id(source.chapter_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                ProjectDuplicateError::InvalidArgument(format!(
                    "独立漫画来源章节不存在: {}",
                    source.chapter_id
                ))
            })?;
        let snapshot = project_source_snapshot::Entity::find_by_id(source.source_snapshot_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                ProjectDuplicateError::InvalidArgument(format!(
                    "独立漫画来源快照不存在: {}",
                    source.source_snapshot_id
                ))
            })?;
        let frozen_chapter =
            source_snapshot_chapter::Entity::find_by_id(source.source_snapshot_chapter_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    ProjectDuplicateError::InvalidArgument(format!(
                        "独立漫画来源快照章节不存在: {}",
                        source.source_snapshot_chapter_id
                    ))
                })?;
        let asset_id = asset_ids
            .get(&source.canvas_text_asset_id)
            .copied()
            .ok_or_else(|| {
                ProjectDuplicateError::InvalidArgument(format!(
                    "独立漫画来源素材不存在: {}",
                    source.canvas_text_asset_id
                ))
            })?;
        let copied_chapter = chapter::ActiveModel {
            project_id: Set(target_project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title.clone()),
            content: Set(chapter.content.clone()),
            word_count: Set(chapter.word_count),
            status: Set(chapter::STATUS_COMPLETED.to_string()),
            summary: Set(chapter.summary.clone()),
            style_tags: Set(chapter.style_tags.clone()),
            analysis_data: Set(Some(serde_json::json!({
                "source_type": "standalone_comic_text",
                "text_asset_id": asset_id,
                "text_asset_version": source.text_asset_version,
                "start_section": source.start_section_index,
                "end_section": source.end_section_index,
                "content_sha256": source.content_hash,
            }))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        let copied_snapshot = project_source_snapshot::ActiveModel {
            root_project_id: Set(target_project_id),
            source_version_hash: Set(snapshot.source_version_hash),
            blueprint_version_id: Set(None),
            chapter_scope_json: Set(snapshot.chapter_scope_json),
            chapter_hashes_json: Set(snapshot.chapter_hashes_json),
            profile_hashes_json: Set(snapshot.profile_hashes_json),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        let copied_frozen_chapter = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(copied_snapshot.id),
            root_project_id: Set(target_project_id),
            source_chapter_id: Set(copied_chapter.id),
            chapter_number: Set(frozen_chapter.chapter_number),
            title: Set(frozen_chapter.title),
            content: Set(frozen_chapter.content),
            summary: Set(frozen_chapter.summary),
            analysis_data_json: Set(copied_chapter.analysis_data.clone()),
            content_hash: Set(frozen_chapter.content_hash),
            analysis_hash: Set(frozen_chapter.analysis_hash),
            ..Default::default()
        }
        .insert(db)
        .await?;
        standalone_comic_text_source::ActiveModel {
            project_id: Set(target_project_id),
            chapter_id: Set(copied_chapter.id),
            source_snapshot_id: Set(copied_snapshot.id),
            source_snapshot_chapter_id: Set(copied_frozen_chapter.id),
            canvas_text_asset_id: Set(asset_id),
            text_asset_version: Set(source.text_asset_version),
            start_section_index: Set(source.start_section_index),
            end_section_index: Set(source.end_section_index),
            source_title: Set(source.source_title),
            content_hash: Set(source.content_hash),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}
