use crate::canvas::CanvasDbError;
use a4n_db_models::models::media::media_binding;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set,
};
use std::collections::BTreeSet;

pub(in crate::canvas::artifacts) async fn ensure_canvas_media_binding<C>(
    db: &C,
    media_asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::Id)
        .filter(media_binding::Column::MediaAssetId.eq(media_asset_id))
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let exists = query.into_tuple::<i64>().one(db).await?.is_some();
    if exists {
        return Ok(());
    }
    media_binding::ActiveModel {
        media_asset_id: Set(media_asset_id),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        role: Set(role.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

pub(super) async fn ensure_canvas_generated_video_bindings<C>(
    db: &C,
    bindings: &[(i64, i64)],
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let bindings = bindings.iter().copied().collect::<BTreeSet<_>>();
    if bindings.is_empty() {
        return Ok(());
    }
    let asset_ids = bindings
        .iter()
        .map(|(asset_id, _)| *asset_id)
        .collect::<BTreeSet<_>>();
    let target_ids = bindings
        .iter()
        .map(|(_, target_id)| *target_id)
        .collect::<BTreeSet<_>>();
    let mut query = media_binding::Entity::find()
        .select_only()
        .column(media_binding::Column::MediaAssetId)
        .column(media_binding::Column::TargetId)
        .filter(media_binding::Column::MediaAssetId.is_in(asset_ids.iter().copied()))
        .filter(media_binding::Column::TargetKind.eq("generated_video"))
        .filter(media_binding::Column::TargetId.is_in(target_ids.iter().copied()))
        .filter(media_binding::Column::Role.eq("generated_result"));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let existing = query
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let now = Utc::now().naive_utc();
    let inserts = bindings
        .difference(&existing)
        .map(|(media_asset_id, target_id)| media_binding::ActiveModel {
            media_asset_id: Set(*media_asset_id),
            target_kind: Set("generated_video".to_string()),
            target_id: Set(*target_id),
            role: Set("generated_result".to_string()),
            created_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    if inserts.is_empty() {
        return Ok(());
    }
    let expected = inserts.len();
    let inserted = media_binding::Entity::insert_many(inserts)
        .exec_without_returning(db)
        .await?;
    if inserted != expected as u64 {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布生成视频绑定批量插入影响 {} 行，预期 {} 行",
            inserted, expected
        )));
    }
    Ok(())
}
