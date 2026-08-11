use a4n_core::models::chapter::chapter;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};

use super::queries::chapter_for_post_process;
use super::{ChapterContentEditError, StoreChapterAnalysisRecordInput};
use crate::project::{ChapterSourceSnapshot, chapter_source_snapshot_matches};

pub async fn chapter_source_snapshots_match_for_update<C>(
    db: &C,
    sources: &[ChapterSourceSnapshot],
) -> Result<bool, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    if sources.is_empty() {
        return Ok(false);
    }

    let mut expected = sources.iter().collect::<Vec<_>>();
    expected.sort_unstable_by_key(|source| source.chapter_id);
    if expected
        .windows(2)
        .any(|pair| pair[0].chapter_id == pair[1].chapter_id)
    {
        return Ok(false);
    }

    let chapter_ids = expected
        .iter()
        .map(|source| source.chapter_id)
        .collect::<Vec<_>>();
    let mut query = chapter::Entity::find()
        .filter(chapter::Column::Id.is_in(chapter_ids))
        .order_by_asc(chapter::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let actual = query.all(db).await?;

    Ok(actual.len() == expected.len()
        && expected
            .into_iter()
            .zip(actual.iter())
            .all(|(source, chapter)| chapter_source_snapshot_matches(source, chapter)))
}

pub async fn lock_chapter_analysis_source<C>(
    db: &C,
    source: &ChapterSourceSnapshot,
) -> Result<chapter::Model, ChapterContentEditError>
where
    C: ConnectionTrait,
{
    let mut query = chapter::Entity::find_by_id(source.chapter_id)
        .filter(chapter::Column::ProjectId.eq(source.project_id));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let chapter = query
        .one(db)
        .await
        .map_err(ChapterContentEditError::database)?
        .ok_or_else(|| {
            ChapterContentEditError::NotFound(format!("章节不存在: {}", source.chapter_id))
        })?;
    if !chapter_source_snapshot_matches(source, &chapter) {
        return Err(ChapterContentEditError::InvalidState {
            current: "chapter_source_changed".to_string(),
            operation: "保存章节结构化分析".to_string(),
        });
    }
    Ok(chapter)
}

pub async fn store_chapter_analysis_record<C>(
    db: &C,
    chapter: chapter::Model,
    input: StoreChapterAnalysisRecordInput,
) -> Result<chapter::Model, ChapterContentEditError>
where
    C: ConnectionTrait,
{
    let mut active: chapter::ActiveModel = chapter.into();
    active.analysis_data = Set(Some(input.analysis_data));
    if let Some(summary) = input.summary {
        active.summary = Set(Some(summary));
    }
    if let Some(style_tags) = input.style_tags {
        active.style_tags = Set(Some(style_tags));
    }
    active
        .update(db)
        .await
        .map_err(ChapterContentEditError::database)
}

pub async fn update_chapter_style_tags_record<C>(
    db: &C,
    chapter_id: i64,
    tags: &[String],
) -> Result<(), ChapterContentEditError>
where
    C: ConnectionTrait,
{
    let chapter = chapter_for_post_process(db, chapter_id).await?;
    let mut active: chapter::ActiveModel = chapter.into();
    active.style_tags = Set(Some(serde_json::json!(tags)));
    active
        .update(db)
        .await
        .map_err(ChapterContentEditError::database)?;
    Ok(())
}
