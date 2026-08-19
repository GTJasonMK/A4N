use a4n_core::models::chapter::{chapter_outline, chapter_outline_provenance};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use super::contracts::ProjectOutlineRecordError;

pub async fn chapter_outline_by_project_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

/// 正文生成只能把用户接管或正式规划流程拥有的章纲当作未来计划。
/// 存在 provenance 的章纲是由已有正文分析得到的回顾资料，不应反过来约束正文。
pub async fn formal_chapter_outline_by_project_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<chapter_outline::Model>, DbErr>
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
        .one(db)
        .await?
        .is_some();
    Ok((!is_derived).then_some(outline))
}

/// 只在正文生成实际读取的正式章纲至提交时仍完全一致时返回其 ID。
///
/// 查询会锁定来源章纲，避免校验通过后、章节提交前又被并发修改。
pub(crate) async fn formal_chapter_outline_id_if_unchanged<C>(
    db: &C,
    source: &chapter_outline::Model,
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
    if current != *source {
        return Ok(None);
    }
    let is_derived = chapter_outline_provenance::Entity::find_by_id(current.id)
        .one(db)
        .await?
        .is_some();
    Ok((!is_derived).then_some(current.id))
}

pub async fn chapter_outline_by_id<C>(
    db: &C,
    outline_id: i64,
) -> Result<Option<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find_by_id(outline_id)
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn chapter_outlines_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn chapter_outlines_for_project_chapter_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}
