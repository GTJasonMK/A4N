use a4n_core::models::canvas::canvas_text_asset;
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::collections::HashMap;

use crate::project::duplicate::ProjectDuplicateError;

pub(in crate::project::duplicate::freeform) async fn copy_freeform_canvas_text_assets_record<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: NaiveDateTime,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let assets = canvas_text_asset::Entity::find()
        .filter(canvas_text_asset::Column::ProjectId.eq(source_project_id))
        .order_by_asc(canvas_text_asset::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let mut id_map = HashMap::new();
    for asset in assets {
        let row = canvas_text_asset::ActiveModel {
            project_id: Set(new_project_id),
            title: Set(asset.title),
            text_kind: Set(asset.text_kind),
            content: Set(asset.content),
            content_hash: Set(asset.content_hash),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
        id_map.insert(asset.id, row.id);
    }
    Ok(id_map)
}
