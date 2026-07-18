use a4n_core::models::chapter::{chapter_outline, part_outline};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};

use super::{ReverseOutlineApplyRow, ReversePartOutlineApplyRow};

pub async fn apply_reverse_outline_records<C>(
    db: &C,
    project_id: i64,
    rows: &[ReverseOutlineApplyRow],
) -> Result<usize, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut outline_count = 0usize;

    for row in rows {
        let existing = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(row.chapter_number))
            .one(db)
            .await?;

        if let Some(existing) = existing {
            let mut active: chapter_outline::ActiveModel = existing.into();
            active.title = Set(Some(row.title.clone()));
            active.summary = Set(Some(row.summary.clone()));
            if let Some(word_count_target) = row.word_count_target {
                active.word_count_target = Set(Some(word_count_target));
            }
            active.status = Set("completed".to_string());
            active.updated_at = Set(now);
            active.update(db).await?;
        } else {
            chapter_outline::ActiveModel {
                project_id: Set(project_id),
                part_id: Set(None),
                chapter_number: Set(row.chapter_number),
                title: Set(Some(row.title.clone())),
                summary: Set(Some(row.summary.clone())),
                beats: Set(None),
                key_scenes: Set(None),
                pacing_score: Set(None),
                foreshadowing_setup: Set(None),
                foreshadowing_payoff: Set(None),
                characters_involved: Set(None),
                word_count_target: Set(row.word_count_target),
                status: Set("completed".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        outline_count += 1;
    }

    Ok(outline_count)
}

pub async fn apply_reverse_outline_records_transaction<T>(
    db: &T,
    project_id: i64,
    rows: &[ReverseOutlineApplyRow],
) -> Result<usize, DbErr>
where
    T: TransactionTrait,
{
    let txn = db.begin().await?;
    let outline_count = apply_reverse_outline_records(&txn, project_id, rows).await?;
    txn.commit().await?;
    Ok(outline_count)
}

pub async fn apply_reverse_part_outline_records<C>(
    db: &C,
    project_id: i64,
    rows: &[ReversePartOutlineApplyRow],
) -> Result<usize, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut part_count = 0usize;

    for row in rows {
        let existing = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .filter(part_outline::Column::PartNumber.eq(row.part_number))
            .one(db)
            .await?;

        if let Some(existing) = existing {
            let mut active: part_outline::ActiveModel = existing.into();
            active.title = Set(row.title.clone());
            active.start_chapter = Set(row.start_chapter);
            active.end_chapter = Set(row.end_chapter);
            active.summary = Set(Some(row.summary.clone()));
            active.theme = Set(Some(row.theme.clone()));
            active.key_events = Set(Some(row.key_events.clone()));
            active.character_focus = Set(Some(row.character_focus.clone()));
            active.updated_at = Set(now);
            active.update(db).await?;
        } else {
            part_outline::ActiveModel {
                project_id: Set(project_id),
                part_number: Set(row.part_number),
                title: Set(row.title.clone()),
                start_chapter: Set(row.start_chapter),
                end_chapter: Set(row.end_chapter),
                summary: Set(Some(row.summary.clone())),
                theme: Set(Some(row.theme.clone())),
                key_events: Set(Some(row.key_events.clone())),
                character_focus: Set(Some(row.character_focus.clone())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        part_count += 1;
    }

    Ok(part_count)
}

pub async fn apply_reverse_part_outline_records_transaction<T>(
    db: &T,
    project_id: i64,
    rows: &[ReversePartOutlineApplyRow],
) -> Result<usize, DbErr>
where
    T: TransactionTrait,
{
    let txn = db.begin().await?;
    let part_count = apply_reverse_part_outline_records(&txn, project_id, rows).await?;
    txn.commit().await?;
    Ok(part_count)
}
