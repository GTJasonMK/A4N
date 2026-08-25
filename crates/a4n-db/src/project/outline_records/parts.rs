use a4n_core::models::chapter::part_outline;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::contracts::ProjectOutlineRecordError;

pub async fn part_outlines_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<part_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn part_outline_by_id<C>(
    db: &C,
    part_id: i64,
) -> Result<Option<part_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    part_outline::Entity::find_by_id(part_id)
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn part_outlines_by_ids<C>(
    db: &C,
    part_ids: &[i64],
) -> Result<Vec<part_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    if part_ids.is_empty() {
        return Ok(Vec::new());
    }
    part_outline::Entity::find()
        .filter(part_outline::Column::Id.is_in(part_ids.iter().copied()))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}
