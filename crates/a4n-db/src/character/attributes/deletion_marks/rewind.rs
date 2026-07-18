use super::super::types::CharacterDeletionMarkRewindSummary;
use super::rules::retained_deletion_mark_count;
use a4n_core::models::character::protagonist_deletion_mark;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
};

pub async fn rewind_deletion_marks_after<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<CharacterDeletionMarkRewindSummary, DbErr>
where
    C: ConnectionTrait,
{
    let marks = protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .filter(protagonist_deletion_mark::Column::LastMarkedChapter.gt(chapter_number))
        .all(db)
        .await?;

    let now = Utc::now().naive_utc();
    let mut summary = CharacterDeletionMarkRewindSummary::default();
    for mark in marks {
        if let Some(retained_count) =
            retained_deletion_mark_count(mark.chapter_number, chapter_number)
        {
            let mut active: protagonist_deletion_mark::ActiveModel = mark.into();
            active.consecutive_count = Set(retained_count);
            active.last_marked_chapter = Set(chapter_number);
            active.is_executed = Set(false);
            active.updated_at = Set(now);
            active.update(db).await?;
            summary.rewound += 1;
        } else {
            protagonist_deletion_mark::Entity::delete_by_id(mark.id)
                .exec(db)
                .await?;
            summary.deleted += 1;
        }
    }

    Ok(summary)
}
