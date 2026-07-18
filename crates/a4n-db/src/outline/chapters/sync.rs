use a4n_core::models::chapter::chapter_outline;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, TransactionTrait,
};

use super::super::types::{ChapterOutlineSyncDraft, OutlineDbError};
use super::records::{insert_chapter_outline_record, update_chapter_outline_record};
use super::rules::index_chapter_outlines_by_number;

/// 按 chapter_number 增量同步章节大纲（更新已有、插入缺失）。
pub async fn sync_chapter_outline_records_by_number_transaction(
    db: &DatabaseConnection,
    project_id: i64,
    part_id: Option<i64>,
    drafts: Vec<ChapterOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<chapter_outline::Model>, OutlineDbError> {
    let txn = db.begin().await?;
    let outlines =
        sync_chapter_outline_records_by_number(&txn, project_id, part_id, drafts, now).await?;
    txn.commit().await?;
    Ok(outlines)
}

/// 按 chapter_number 增量同步章节大纲（更新已有、插入缺失）。
///
/// 调用方已经持有事务时使用该入口；普通业务写入应使用
/// [`sync_chapter_outline_records_by_number_transaction`]。
pub async fn sync_chapter_outline_records_by_number<C>(
    db: &C,
    project_id: i64,
    part_id: Option<i64>,
    drafts: Vec<ChapterOutlineSyncDraft>,
    now: NaiveDateTime,
) -> Result<Vec<chapter_outline::Model>, OutlineDbError>
where
    C: ConnectionTrait,
{
    let chapter_numbers = drafts
        .iter()
        .map(|draft| draft.chapter_number)
        .collect::<Vec<_>>();

    let existing = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.is_in(chapter_numbers))
        .all(db)
        .await?;
    let mut existing_by_number = index_chapter_outlines_by_number(existing)?;

    let mut results = Vec::with_capacity(drafts.len());
    for draft in drafts {
        let outline = if let Some(existing) = existing_by_number.remove(&draft.chapter_number) {
            update_chapter_outline_record(db, existing, part_id, &draft, now).await?
        } else {
            insert_chapter_outline_record(db, project_id, part_id, &draft, now).await?
        };
        results.push(outline);
    }

    Ok(results)
}
