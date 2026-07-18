use crate::galgame::{GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE, GalgameMediaBindingReadModel};
use a4n_core::models::media::{media_asset, media_binding};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeSet;

pub async fn galgame_media_binding_read_model(
    db: &DatabaseConnection,
    scene_ids: &[i64],
    line_ids: &[i64],
) -> Result<GalgameMediaBindingReadModel, DbErr> {
    if scene_ids.is_empty() && line_ids.is_empty() {
        return Ok(GalgameMediaBindingReadModel::default());
    }
    let mut condition = sea_orm::Condition::any();
    if !scene_ids.is_empty() {
        condition = condition.add(
            sea_orm::Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_SCENE))
                .add(media_binding::Column::TargetId.is_in(scene_ids.to_vec())),
        );
    }
    if !line_ids.is_empty() {
        condition = condition.add(
            sea_orm::Condition::all()
                .add(media_binding::Column::TargetKind.eq(GALGAME_ENTITY_LINE))
                .add(media_binding::Column::TargetId.is_in(line_ids.to_vec())),
        );
    }
    let bindings = media_binding::Entity::find()
        .filter(condition)
        .order_by_asc(media_binding::Column::TargetKind)
        .order_by_asc(media_binding::Column::TargetId)
        .order_by_asc(media_binding::Column::Role)
        .order_by_asc(media_binding::Column::Id)
        .all(db)
        .await?;
    if bindings.is_empty() {
        return Ok(GalgameMediaBindingReadModel::default());
    }
    let asset_ids = bindings
        .iter()
        .map(|binding| binding.media_asset_id)
        .collect::<BTreeSet<_>>();
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::Id.is_in(asset_ids))
        .all(db)
        .await?;
    Ok(GalgameMediaBindingReadModel { bindings, assets })
}
