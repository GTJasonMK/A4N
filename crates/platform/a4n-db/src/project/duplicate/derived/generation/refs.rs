use std::collections::HashMap;

use a4n_db_models::models::generation::generation_spec_media_ref;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use crate::project::duplicate::{ProjectDuplicateError, shared::remap_derived_required_id};

pub(in crate::project::duplicate) async fn copy_derived_generation_spec_media_refs<
    C: ConnectionTrait,
>(
    db: &C,
    old_spec_ids: &[i64],
    spec_id_map: &HashMap<i64, i64>,
    asset_id_map: &HashMap<i64, i64>,
) -> Result<(), ProjectDuplicateError> {
    if old_spec_ids.is_empty() {
        return Ok(());
    }
    let refs = generation_spec_media_ref::Entity::find()
        .filter(generation_spec_media_ref::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .all(db)
        .await?;
    let inserts = refs
        .into_iter()
        .map(|item| {
            Ok(generation_spec_media_ref::ActiveModel {
                generation_spec_id: Set(remap_derived_required_id(
                    spec_id_map,
                    item.generation_spec_id,
                    "generation_spec_media_ref.generation_spec_id",
                )?),
                media_asset_id: Set(remap_derived_required_id(
                    asset_id_map,
                    item.media_asset_id,
                    "generation_spec_media_ref.media_asset_id",
                )?),
                role: Set(item.role),
            })
        })
        .collect::<Result<Vec<_>, ProjectDuplicateError>>()?;
    if !inserts.is_empty() {
        generation_spec_media_ref::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await?;
    }
    Ok(())
}
