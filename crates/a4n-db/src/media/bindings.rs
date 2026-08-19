use a4n_core::models::media::{media_asset, media_binding};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};
use std::collections::BTreeSet;

pub(crate) async fn delete_generated_image_target_media_bindings<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    delete_generated_target_media_bindings(
        db,
        CanvasNodeRefKind::GeneratedImage.as_str(),
        image_ids,
    )
    .await
}

pub(crate) async fn delete_generated_video_target_media_bindings<C>(
    db: &C,
    video_ids: &[i64],
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    delete_generated_target_media_bindings(
        db,
        CanvasNodeRefKind::GeneratedVideo.as_str(),
        video_ids,
    )
    .await
}

pub(crate) async fn generated_video_target_media_binding_project_ids<C>(
    db: &C,
    video_ids: &[i64],
) -> Result<BTreeSet<i64>, DbErr>
where
    C: ConnectionTrait,
{
    let target_ids = normalized_generated_target_ids(video_ids);
    if target_ids.is_empty() {
        return Ok(BTreeSet::new());
    }
    let media_asset_ids = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .filter(media_binding::Column::TargetKind.eq(CanvasNodeRefKind::GeneratedVideo.as_str()))
        .filter(media_binding::Column::TargetId.is_in(target_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?;
    if media_asset_ids.is_empty() {
        return Ok(BTreeSet::new());
    }

    Ok(media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::ProjectId)
        .filter(media_asset::Column::Id.is_in(media_asset_ids))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect())
}

async fn delete_generated_target_media_bindings<C>(
    db: &C,
    target_kind: &str,
    target_ids: &[i64],
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    let target_ids = normalized_generated_target_ids(target_ids);
    if target_ids.is_empty() {
        return Ok(0);
    }

    Ok(media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.is_in(target_ids))
        .exec(db)
        .await?
        .rows_affected)
}

pub(super) fn normalized_generated_target_ids(target_ids: &[i64]) -> Vec<i64> {
    let mut target_ids = target_ids
        .iter()
        .copied()
        .filter(|target_id| *target_id > 0)
        .collect::<Vec<_>>();
    target_ids.sort_unstable();
    target_ids.dedup();
    target_ids
}
