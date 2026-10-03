use super::validation::ensure_text_root_project;
use super::{
    CurrentChapterContentAnchorRows, CurrentChapterContentSource, SOURCE_KIND_CHAPTER_CONTENT,
    TextAnchorContentInsertInput, TextAnchorDbError, TextAnchorDbResult,
    insert_content_anchor_records,
};
use a4n_db_models::models::{chapter::chapter, project::text_anchor};
use a4n_domain::source_text_hash;
use a4n_domain::text_anchor::TextAnchorRecord;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use std::collections::{HashMap, HashSet};

fn text_anchor_record(model: text_anchor::Model) -> TextAnchorRecord {
    TextAnchorRecord {
        id: model.id,
        project_id: model.project_id,
        source_chapter_id: model.source_chapter_id,
        source_snapshot_id: model.source_snapshot_id,
        source_snapshot_chapter_id: model.source_snapshot_chapter_id,
        source_kind: model.source_kind,
        source_id: model.source_id,
        paragraph_index: model.paragraph_index,
        start_offset: model.start_offset,
        end_offset: model.end_offset,
        quote: model.quote,
        quote_hash: model.quote_hash,
        source_version_hash: model.source_version_hash,
        created_at: model.created_at,
    }
}

pub async fn ensure_chapter_content_anchor_records_batch_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_ids: &[i64],
) -> TextAnchorDbResult<Vec<Vec<TextAnchorRecord>>> {
    ensure_chapter_content_anchor_records_batch(db, project_id, chapter_ids)
        .await
        .map(|groups| {
            groups
                .into_iter()
                .map(|group| group.into_iter().map(text_anchor_record).collect())
                .collect()
        })
}

pub async fn read_current_chapter_content_anchor_records_batch_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_ids: &[i64],
) -> TextAnchorDbResult<Vec<super::CurrentChapterContentAnchorRecordRows>> {
    read_current_chapter_content_anchor_records_batch(db, project_id, chapter_ids)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| super::CurrentChapterContentAnchorRecordRows {
                    chapter: row.chapter,
                    anchors: row.anchors.into_iter().map(text_anchor_record).collect(),
                    source_version_hash: row.source_version_hash,
                })
                .collect()
        })
}

pub async fn delete_chapter_content_anchor_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_ids: &[i64],
) -> TextAnchorDbResult<()> {
    super::delete_chapter_content_profile_evidence_records_batch(db, project_id, chapter_ids)
        .await?;
    text_anchor::Entity::delete_many()
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.is_in(chapter_ids.iter().copied()))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .exec(db)
        .await?;
    Ok(())
}

pub async fn rebuild_chapter_content_anchor_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> TextAnchorDbResult<Vec<text_anchor::Model>> {
    ensure_text_root_project(db, project_id).await?;
    rebuild_chapter_content_anchor_records_for_validated_project(db, project_id, chapter_id).await
}

async fn rebuild_chapter_content_anchor_records_for_validated_project(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> TextAnchorDbResult<Vec<text_anchor::Model>> {
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ProjectId)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .into_model::<CurrentChapterContentSource>()
        .one(db)
        .await?
        .ok_or_else(|| TextAnchorDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
    if chapter.project_id != project_id {
        return Err(TextAnchorDbError::InvalidArgument(format!(
            "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
        )));
    }

    let tx = db.begin().await?;
    delete_chapter_content_anchor_records(&tx, project_id, &[chapter.id]).await?;

    let source_version_hash = source_text_hash(&chapter.content);
    let anchors = insert_content_anchor_records(
        &tx,
        TextAnchorContentInsertInput {
            project_id,
            source_chapter_id: Some(chapter.id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_id: chapter.id,
            chapter_number: chapter.chapter_number,
            content: &chapter.content,
            source_version_hash: &source_version_hash,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(anchors)
}

pub async fn ensure_chapter_content_anchor_records_batch(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_ids: &[i64],
) -> TextAnchorDbResult<Vec<Vec<text_anchor::Model>>> {
    if chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows =
        read_current_chapter_content_anchor_records_batch(db, project_id, chapter_ids).await?;
    let mut pending_rebuilds = Vec::new();
    let mut pending_ids = HashSet::new();
    let mut anchors_by_chapter = Vec::with_capacity(rows.len());
    for row in rows {
        let current = !row.anchors.is_empty()
            && row
                .anchors
                .iter()
                .all(|anchor| anchor.source_version_hash == row.source_version_hash);
        let chapter_id = row.chapter.id;
        if current {
            anchors_by_chapter.push((chapter_id, Some(row.anchors)));
        } else {
            if pending_ids.insert(chapter_id) {
                pending_rebuilds.push(row);
            }
            anchors_by_chapter.push((chapter_id, None));
        }
    }
    if pending_rebuilds.is_empty() {
        return anchors_by_chapter
            .into_iter()
            .map(|(_, anchors)| {
                anchors.ok_or_else(|| {
                    TextAnchorDbError::InvalidArgument("正文锚点批量结果缺少章节记录".to_string())
                })
            })
            .collect();
    }

    let pending_ids = pending_rebuilds
        .iter()
        .map(|row| row.chapter.id)
        .collect::<Vec<_>>();
    let tx = db.begin().await?;
    let latest_chapters = chapter::Entity::find_current()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ProjectId)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .filter(chapter::Column::Id.is_in(pending_ids.iter().copied()))
        .into_model::<CurrentChapterContentSource>()
        .all(&tx)
        .await?
        .into_iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    for chapter_id in &pending_ids {
        let chapter = latest_chapters
            .get(chapter_id)
            .ok_or_else(|| TextAnchorDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if chapter.project_id != project_id {
            return Err(TextAnchorDbError::InvalidArgument(format!(
                "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
            )));
        }
    }

    delete_chapter_content_anchor_records(&tx, project_id, &pending_ids).await?;

    let source_hashes = pending_rebuilds
        .iter()
        .map(|row| {
            latest_chapters
                .get(&row.chapter.id)
                .map(|chapter| source_text_hash(&chapter.content))
                .ok_or_else(|| {
                    TextAnchorDbError::NotFound(format!("章节不存在: {}", row.chapter.id))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inputs = pending_rebuilds
        .iter()
        .zip(source_hashes.iter())
        .map(|(row, source_version_hash)| {
            let chapter = latest_chapters.get(&row.chapter.id).ok_or_else(|| {
                TextAnchorDbError::NotFound(format!("章节不存在: {}", row.chapter.id))
            })?;
            Ok(TextAnchorContentInsertInput {
                project_id,
                source_chapter_id: Some(chapter.id),
                source_snapshot_id: None,
                source_snapshot_chapter_id: None,
                source_id: chapter.id,
                chapter_number: chapter.chapter_number,
                content: &chapter.content,
                source_version_hash,
            })
        })
        .collect::<Result<Vec<_>, TextAnchorDbError>>()?;
    let rebuilt = super::insert_content_anchor_records_batch(&tx, &inputs).await?;
    tx.commit().await?;

    let rebuilt_by_chapter = pending_rebuilds
        .iter()
        .map(|row| row.chapter.id)
        .zip(rebuilt)
        .collect::<HashMap<_, _>>();
    anchors_by_chapter
        .into_iter()
        .map(|(chapter_id, anchors)| {
            anchors
                .or_else(|| rebuilt_by_chapter.get(&chapter_id).cloned())
                .ok_or_else(|| {
                    TextAnchorDbError::InvalidArgument(format!(
                        "正文锚点批量结果缺少章节记录: chapter_id={chapter_id}"
                    ))
                })
        })
        .collect()
}

pub async fn read_current_chapter_content_anchor_records_batch(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_ids: &[i64],
) -> TextAnchorDbResult<Vec<CurrentChapterContentAnchorRows>> {
    if chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    ensure_text_root_project(db, project_id).await?;
    let chapters = chapter::Entity::find_current()
        .select_only()
        .column(chapter::Column::Id)
        .column(chapter::Column::ProjectId)
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .filter(chapter::Column::Id.is_in(chapter_ids.iter().copied()))
        .into_model::<CurrentChapterContentSource>()
        .all(db)
        .await?;
    let chapters_by_id = chapters
        .into_iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<HashMap<_, _>>();
    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::ProjectId.eq(project_id))
        .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.is_in(chapter_ids.iter().copied()))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .order_by_asc(text_anchor::Column::SourceId)
        .order_by_asc(text_anchor::Column::StartOffset)
        .all(db)
        .await?;
    let mut anchors_by_chapter = HashMap::<i64, Vec<text_anchor::Model>>::new();
    for anchor in anchors {
        anchors_by_chapter
            .entry(anchor.source_id)
            .or_default()
            .push(anchor);
    }
    chapter_ids
        .iter()
        .map(|chapter_id| {
            let chapter = chapters_by_id
                .get(chapter_id)
                .cloned()
                .ok_or_else(|| TextAnchorDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
            if chapter.project_id != project_id {
                return Err(TextAnchorDbError::InvalidArgument(format!(
                    "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
                )));
            }
            Ok(CurrentChapterContentAnchorRows {
                source_version_hash: source_text_hash(&chapter.content),
                anchors: anchors_by_chapter.remove(chapter_id).unwrap_or_default(),
                chapter,
            })
        })
        .collect()
}
