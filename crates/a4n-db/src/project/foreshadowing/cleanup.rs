use a4n_core::models::character::foreshadowing_index;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::contracts::{ForeshadowingCleanupResult, ForeshadowingDbError};

pub async fn cleanup_foreshadowing_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<ForeshadowingCleanupResult, ForeshadowingDbError>
where
    C: ConnectionTrait,
{
    let mut result = ForeshadowingCleanupResult::default();

    let setup_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.eq(chapter_number))
        .all(db)
        .await
        .map_err(ForeshadowingDbError::Database)?;
    for item in setup_foreshadowing {
        result.stale_ids.push(item.id);
        foreshadowing_index::Entity::delete_by_id(item.id)
            .exec(db)
            .await
            .map_err(ForeshadowingDbError::Database)?;
    }

    let revealed_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::PayoffChapter.eq(Some(chapter_number)))
        .all(db)
        .await
        .map_err(ForeshadowingDbError::Database)?;
    for item in revealed_foreshadowing {
        result.changed_ids.push(item.id);
        let mut active: foreshadowing_index::ActiveModel = item.into();
        active.payoff_chapter = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(db)
            .await
            .map_err(ForeshadowingDbError::Database)?;
    }

    Ok(result)
}
