use a4n_db_models::models::chapter::{chapter_outline, chapter_outline_provenance};
use a4n_domain::chapter::ChapterOutlineRecord;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use super::contracts::ProjectOutlineRecordError;
use crate::outline::record_mapping::chapter_outline_record_from_model;

pub async fn chapter_outline_by_project_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<ChapterOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let model = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(model.map(chapter_outline_record_from_model))
}

/// 正文生成只能把用户接管或正式规划流程拥有的章纲当作未来计划。
/// 存在 provenance 的章纲是由已有正文分析得到的回顾资料，不应反过来约束正文。
pub async fn formal_chapter_outline_by_project_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<ChapterOutlineRecord>, DbErr>
where
    C: ConnectionTrait,
{
    let Some(outline) = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let is_derived = chapter_outline_provenance::Entity::find_by_id(outline.id)
        .select_only()
        .column(chapter_outline_provenance::Column::OutlineId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some();
    Ok((!is_derived).then(|| chapter_outline_record_from_model(outline)))
}

/// 只在正文生成实际读取的正式章纲至提交时仍完全一致时返回其 ID。
///
/// 查询会锁定来源章纲，避免校验通过后、章节提交前又被并发修改。
pub(crate) async fn formal_chapter_outline_id_if_unchanged<C>(
    db: &C,
    source: &ChapterOutlineRecord,
) -> Result<Option<i64>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = chapter_outline::Entity::find_by_id(source.id)
        .filter(chapter_outline::Column::ProjectId.eq(source.project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(source.chapter_number));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let Some(current) = query.one(db).await? else {
        return Ok(None);
    };
    let current_id = current.id;
    if chapter_outline_record_from_model(current) != *source {
        return Ok(None);
    }
    let is_derived = chapter_outline_provenance::Entity::find_by_id(current_id)
        .select_only()
        .column(chapter_outline_provenance::Column::OutlineId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some();
    Ok((!is_derived).then_some(current_id))
}

pub async fn chapter_outline_by_id<C>(
    db: &C,
    outline_id: i64,
) -> Result<Option<ChapterOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let model = chapter_outline::Entity::find_by_id(outline_id)
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(model.map(chapter_outline_record_from_model))
}

pub async fn chapter_outlines_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<ChapterOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let models = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(models
        .into_iter()
        .map(chapter_outline_record_from_model)
        .collect())
}

pub async fn chapter_outline_index_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<super::contracts::ChapterOutlineListRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let rows = chapter_outline::Entity::find()
        .select_only()
        .columns([
            chapter_outline::Column::Id,
            chapter_outline::Column::ProjectId,
            chapter_outline::Column::ChapterNumber,
            chapter_outline::Column::Title,
            chapter_outline::Column::Summary,
            chapter_outline::Column::WordCountTarget,
            chapter_outline::Column::UpdatedAt,
        ])
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .into_tuple::<(
            i64,
            i64,
            i32,
            Option<String>,
            Option<String>,
            Option<i32>,
            NaiveDateTime,
        )>()
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    let ids = rows.iter().map(|row| row.0).collect::<Vec<_>>();
    let source_kinds = super::provenance::chapter_outline_source_kinds_for_ids(db, &ids).await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, project_id, chapter_number, title, summary, word_count_target, updated_at)| {
                super::contracts::ChapterOutlineListRecord {
                    id,
                    project_id,
                    chapter_number,
                    title,
                    summary,
                    word_count_target,
                    source_kind: source_kinds.get(&id).cloned(),
                    updated_at,
                }
            },
        )
        .collect())
}

pub async fn chapter_outlines_for_project_chapter_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<ChapterOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let models = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(models
        .into_iter()
        .map(chapter_outline_record_from_model)
        .collect())
}
