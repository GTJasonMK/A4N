use super::super::types::CharacterDeletionMarkUpsertInput;
use super::rules::next_deletion_mark_count;
use a4n_db_models::models::character::protagonist_deletion_mark;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
};

pub(super) async fn upsert_deletion_mark<C>(
    db: &C,
    input: CharacterDeletionMarkUpsertInput,
) -> Result<protagonist_deletion_mark::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let existing = protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(input.profile_id))
        .filter(protagonist_deletion_mark::Column::Category.eq(&input.category))
        .filter(protagonist_deletion_mark::Column::Key.eq(&input.key))
        .one(db)
        .await?;

    if let Some(mark) = existing {
        if mark.is_executed {
            protagonist_deletion_mark::Entity::delete_by_id(mark.id)
                .exec(db)
                .await?;
        } else {
            let mut active: protagonist_deletion_mark::ActiveModel = mark.clone().into();
            let next_count = next_deletion_mark_count(
                mark.consecutive_count,
                mark.last_marked_chapter,
                input.chapter_number,
            );
            if input.chapter_number != mark.last_marked_chapter + 1 {
                active.chapter_number = Set(input.chapter_number);
            }
            active.consecutive_count = Set(next_count);
            active.last_marked_chapter = Set(input.chapter_number);
            active.source_kind = Set(input.source_kind);
            active.reason = Set(input.reason);
            active.evidence = Set(input.evidence);
            active.updated_at = Set(now);
            return active.update(db).await;
        }
    }

    protagonist_deletion_mark::ActiveModel {
        profile_id: Set(input.profile_id),
        source_kind: Set(input.source_kind),
        category: Set(input.category),
        key: Set(input.key),
        chapter_number: Set(input.chapter_number),
        reason: Set(input.reason),
        evidence: Set(input.evidence),
        consecutive_count: Set(1),
        is_executed: Set(false),
        last_marked_chapter: Set(input.chapter_number),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
}
