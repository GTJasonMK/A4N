use super::super::types::CharacterDeletionMarkRewindSummary;
use super::rules::retained_deletion_mark_count;
use a4n_db_models::models::character::{
    attribute_event::{SOURCE_KIND_AI_SYNC, SOURCE_KIND_TEXT_FACT},
    protagonist_deletion_mark,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    sea_query::{CaseStatement, Expr},
};

pub async fn rewind_deletion_marks_after<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<CharacterDeletionMarkRewindSummary, DbErr>
where
    C: ConnectionTrait,
{
    rewind_deletion_marks_after_with_source(db, profile_id, chapter_number, None).await
}

pub async fn rewind_ai_deletion_marks_after<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
) -> Result<CharacterDeletionMarkRewindSummary, DbErr>
where
    C: ConnectionTrait,
{
    rewind_deletion_marks_after_with_source(
        db,
        profile_id,
        chapter_number,
        Some(SOURCE_KIND_AI_SYNC),
    )
    .await
}

async fn rewind_deletion_marks_after_with_source<C>(
    db: &C,
    profile_id: i64,
    chapter_number: i32,
    source_kind: Option<&str>,
) -> Result<CharacterDeletionMarkRewindSummary, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .filter(protagonist_deletion_mark::Column::LastMarkedChapter.gt(chapter_number));
    if let Some(source_kind) = source_kind {
        query = query.filter(
            protagonist_deletion_mark::Column::SourceKind
                .is_in([source_kind, SOURCE_KIND_TEXT_FACT]),
        );
    }
    let marks = query.all(db).await?;

    let now = Utc::now().naive_utc();
    let mut rewound_ids = Vec::new();
    let mut rewound_counts = CaseStatement::new();
    let mut deleted_ids = Vec::new();
    for mark in marks {
        if let Some(retained_count) =
            retained_deletion_mark_count(mark.chapter_number, chapter_number)
        {
            rewound_counts = rewound_counts.case(
                protagonist_deletion_mark::Column::Id.eq(mark.id),
                Expr::value(retained_count),
            );
            rewound_ids.push(mark.id);
        } else {
            deleted_ids.push(mark.id);
        }
    }

    if !rewound_ids.is_empty() {
        let updated = protagonist_deletion_mark::Entity::update_many()
            .col_expr(
                protagonist_deletion_mark::Column::ConsecutiveCount,
                rewound_counts
                    .finally(Expr::col(
                        protagonist_deletion_mark::Column::ConsecutiveCount,
                    ))
                    .into(),
            )
            .col_expr(
                protagonist_deletion_mark::Column::LastMarkedChapter,
                Expr::value(chapter_number),
            )
            .col_expr(
                protagonist_deletion_mark::Column::IsExecuted,
                Expr::value(false),
            )
            .col_expr(
                protagonist_deletion_mark::Column::UpdatedAt,
                Expr::value(now),
            )
            .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
            .filter(protagonist_deletion_mark::Column::Id.is_in(rewound_ids.iter().copied()))
            .exec(db)
            .await?;
        if updated.rows_affected != rewound_ids.len() as u64 {
            return Err(DbErr::Custom(format!(
                "回退角色删除标记影响 {} 行，预期 {} 行",
                updated.rows_affected,
                rewound_ids.len()
            )));
        }
    }
    if !deleted_ids.is_empty() {
        protagonist_deletion_mark::Entity::delete_many()
            .filter(protagonist_deletion_mark::Column::Id.is_in(deleted_ids.iter().copied()))
            .exec(db)
            .await?;
    }

    Ok(CharacterDeletionMarkRewindSummary {
        rewound: rewound_ids.len() as u64,
        deleted: deleted_ids.len() as u64,
    })
}
