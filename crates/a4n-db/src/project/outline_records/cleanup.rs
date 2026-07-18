use a4n_core::models::chapter::chapter_outline;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::project::ProjectCleanupDbError;

pub async fn delete_chapter_outlines_by_ids<C>(
    db: &C,
    outline_ids: &[i64],
) -> Result<(), ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    if outline_ids.is_empty() {
        return Ok(());
    }
    chapter_outline::Entity::delete_many()
        .filter(chapter_outline::Column::Id.is_in(outline_ids.iter().copied()))
        .exec(db)
        .await?;
    Ok(())
}
