use std::collections::HashMap;

use a4n_db_models::models::media::media_asset;
use a4n_domain::project::ProjectMediaFileCopier;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::project::duplicate::{ProjectDuplicateError, shared::remap_derived_optional_id};

use super::{
    super::types::DuplicatedDerivedSourceSnapshot, rules::ensure_copyable_derived_media_asset,
};

pub(in crate::project::duplicate::derived) async fn copy_derived_media_assets<C, Copier>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    root_project_id: i64,
    duplicated_snapshot: &DuplicatedDerivedSourceSnapshot,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier + ?Sized,
{
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(source_project_id))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if assets.is_empty() {
        return Ok(HashMap::new());
    }
    let mut rows = Vec::with_capacity(assets.len());
    for asset in &assets {
        ensure_copyable_derived_media_asset(asset)?;
        let source_snapshot_chapter_id = remap_derived_optional_id(
            &duplicated_snapshot.chapter_ids,
            asset.source_snapshot_chapter_id,
            "media_asset.source_snapshot_chapter_id",
        )?;
        let file_path = media_file_copier
            .copy_media_file(new_project_id, asset.id, "file", asset.file_path.as_deref())
            .map_err(ProjectDuplicateError::MediaCopy)?;
        let thumbnail_path = media_file_copier
            .copy_media_file(
                new_project_id,
                asset.id,
                "thumbnail",
                asset.thumbnail_path.as_deref(),
            )
            .map_err(ProjectDuplicateError::MediaCopy)?;
        rows.push(media_asset::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(root_project_id),
            source_snapshot_id: Set(Some(duplicated_snapshot.snapshot.id)),
            source_chapter_id: Set(asset.source_chapter_id),
            source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
            media_kind: Set(asset.media_kind.clone()),
            origin: Set(asset.origin.clone()),
            file_path: Set(file_path),
            thumbnail_path: Set(thumbnail_path),
            width: Set(asset.width),
            height: Set(asset.height),
            duration_secs: Set(asset.duration_secs),
            fps: Set(asset.fps),
            file_size: Set(asset.file_size),
            checksum: Set(asset.checksum.clone()),
            status: Set(asset.status.clone()),
            error_message: Set(asset.error_message.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });
    }
    let inserted = media_asset::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != assets.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的媒体资产数量与源记录不一致".to_string(),
            operation: "复制派生项目媒体资产".to_string(),
        });
    }
    let mut asset_id_map = HashMap::with_capacity(assets.len());
    for (asset, copied) in assets.iter().zip(inserted) {
        asset_id_map.insert(asset.id, copied.id);
    }
    Ok(asset_id_map)
}
