use a4n_db_models::models::chapter::part_outline;
use a4n_domain::chapter::PartOutlineRecord;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::outline::record_mapping::part_outline_record_from_model;

use super::contracts::ProjectOutlineRecordError;

pub async fn part_outlines_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<PartOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let models = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(models
        .into_iter()
        .map(part_outline_record_from_model)
        .collect())
}

pub async fn part_outline_by_id<C>(
    db: &C,
    part_id: i64,
) -> Result<Option<PartOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    let model = part_outline::Entity::find_by_id(part_id)
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(model.map(part_outline_record_from_model))
}

pub async fn part_outlines_by_ids<C>(
    db: &C,
    part_ids: &[i64],
) -> Result<Vec<PartOutlineRecord>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    if part_ids.is_empty() {
        return Ok(Vec::new());
    }
    let models = part_outline::Entity::find()
        .filter(part_outline::Column::Id.is_in(part_ids.iter().copied()))
        .order_by_asc(part_outline::Column::PartNumber)
        .order_by_asc(part_outline::Column::Id)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)?;
    Ok(models
        .into_iter()
        .map(part_outline_record_from_model)
        .collect())
}
