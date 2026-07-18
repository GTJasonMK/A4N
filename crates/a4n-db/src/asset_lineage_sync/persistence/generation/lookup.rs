use a4n_core::models::generation::generation_spec;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;

use crate::asset_lineage_sync::{AssetLineageSyncError, types::FamilyLineageContext};

pub(super) async fn find_generation_spec_by_key<C>(
    db: &C,
    family: &FamilyLineageContext,
    canonical_key: &str,
) -> Result<Option<generation_spec::Model>, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    Ok(rows.into_iter().find(|row| {
        row.params_json.get("canonical_key").and_then(Value::as_str) == Some(canonical_key)
    }))
}
