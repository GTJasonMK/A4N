use std::collections::BTreeMap;

use a4n_core::models::media::{media_asset, media_provenance};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::Value;

use crate::asset_lineage_sync::{AssetLineageSyncError, FamilyLineageContext};

pub(super) async fn find_media_asset_by_key<C>(
    db: &C,
    family: &FamilyLineageContext,
    canonical_key: &str,
) -> Result<Option<(media_asset::Model, media_provenance::Model)>, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    if assets.is_empty() {
        return Ok(None);
    }
    let asset_by_id = assets
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect::<BTreeMap<_, _>>();
    let provenances = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(asset_by_id.keys().copied()))
        .order_by_asc(media_provenance::Column::MediaAssetId)
        .all(db)
        .await
        .map_err(AssetLineageSyncError::Database)?;
    for provenance in provenances {
        let matches = provenance
            .request_snapshot_json
            .as_ref()
            .and_then(|value| value.get("canonical_key"))
            .and_then(Value::as_str)
            == Some(canonical_key);
        if matches && let Some(asset) = asset_by_id.get(&provenance.media_asset_id) {
            return Ok(Some((asset.clone(), provenance)));
        }
    }
    Ok(None)
}
