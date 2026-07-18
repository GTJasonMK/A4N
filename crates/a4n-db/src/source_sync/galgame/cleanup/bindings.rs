use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::media::media_binding;
use sea_orm::{ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

pub async fn bound_media_asset_ids_for_galgame_scene<C>(
    db: &C,
    scene_id: i64,
    line_ids: &[i64],
) -> Result<Vec<i64>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(scene_id, "scene_id")?;
    for line_id in line_ids {
        validate_positive_id(*line_id, "line_id")?;
    }

    media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(galgame_scene_media_binding_condition(scene_id, line_ids))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(SourceSyncDbError::from)
}

pub async fn delete_media_binding_records_for_galgame_scene<C>(
    db: &C,
    scene_id: i64,
    line_ids: &[i64],
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(scene_id, "scene_id")?;
    for line_id in line_ids {
        validate_positive_id(*line_id, "line_id")?;
    }

    media_binding::Entity::delete_many()
        .filter(galgame_scene_media_binding_condition(scene_id, line_ids))
        .exec(db)
        .await?;
    Ok(())
}

fn galgame_scene_media_binding_condition(scene_id: i64, line_ids: &[i64]) -> Condition {
    let mut condition = Condition::any().add(
        Condition::all()
            .add(media_binding::Column::TargetKind.eq("galgame_scene"))
            .add(media_binding::Column::TargetId.eq(scene_id)),
    );
    if !line_ids.is_empty() {
        condition = condition.add(
            Condition::all()
                .add(media_binding::Column::TargetKind.eq("galgame_line"))
                .add(media_binding::Column::TargetId.is_in(line_ids.to_vec())),
        );
    }
    condition
}
