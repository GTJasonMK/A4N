use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use chrono::NaiveDateTime;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::super::ranges::expected_part_id_for_existing_outline;
use super::super::types::OutlineDbError;

/// 按当前分部范围重绑章节大纲的 part_id。
pub async fn sync_chapter_outline_part_assignments<C>(
    db: &C,
    project_id: i64,
    parts: &[part_outline::Model],
    now: NaiveDateTime,
) -> Result<(), OutlineDbError>
where
    C: ConnectionTrait,
{
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await?;

    let mut part_id_case = CaseStatement::new();
    let mut updated_at_case = CaseStatement::new();
    let mut changed_ids = Vec::new();
    for outline in outlines {
        let expected_part_id = expected_part_id_for_existing_outline(parts, &outline)?;
        if outline.part_id == expected_part_id {
            continue;
        }
        let updated_at = std::cmp::max(now, outline.updated_at + chrono::Duration::microseconds(1));
        part_id_case = part_id_case.case(
            chapter_outline::Column::Id.eq(outline.id),
            Expr::value(expected_part_id),
        );
        updated_at_case = updated_at_case.case(
            chapter_outline::Column::Id.eq(outline.id),
            Expr::value(updated_at),
        );
        changed_ids.push(outline.id);
    }

    if changed_ids.is_empty() {
        return Ok(());
    }
    let updated = chapter_outline::Entity::update_many()
        .col_expr(
            chapter_outline::Column::PartId,
            part_id_case
                .finally(Expr::col(chapter_outline::Column::PartId))
                .into(),
        )
        .col_expr(
            chapter_outline::Column::UpdatedAt,
            updated_at_case
                .finally(Expr::col(chapter_outline::Column::UpdatedAt))
                .into(),
        )
        .filter(chapter_outline::Column::Id.is_in(changed_ids.clone()))
        .exec(db)
        .await?;
    if updated.rows_affected != changed_ids.len() as u64 {
        return Err(OutlineDbError::InvalidState {
            current: format!(
                "章节大纲分部归属回填影响 {} 行，预期 {} 行",
                updated.rows_affected,
                changed_ids.len()
            ),
            operation: "同步章节大纲分部归属".to_string(),
        });
    }

    Ok(())
}
