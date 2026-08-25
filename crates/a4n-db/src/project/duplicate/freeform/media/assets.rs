use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::shared::ProjectMediaFileCopier;
use a4n_core::models::media::media_asset;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::HashMap;

pub(super) async fn copy_freeform_media_assets_record<C, Copier>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: NaiveDateTime,
    media_file_copier: &mut Copier,
) -> Result<HashMap<i64, i64>, ProjectDuplicateError>
where
    C: ConnectionTrait,
    Copier: ProjectMediaFileCopier,
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
    let rows = assets
        .iter()
        .map(|asset| {
            if asset.root_project_id != source_project_id
                || asset.source_chapter_id.is_some()
                || asset.source_snapshot_chapter_id.is_some()
            {
                return Err(ProjectDuplicateError::InvalidState {
                    current: format!("媒体资产 {} 包含派生项目上下文", asset.id),
                    operation: "复制自由视频项目媒体资产".to_string(),
                });
            }
            if asset.status == "completed" && asset.file_path.is_none() {
                return Err(ProjectDuplicateError::InvalidState {
                    current: format!("已完成媒体资产 {} 缺少 file_path", asset.id),
                    operation: "复制自由视频项目媒体资产".to_string(),
                });
            }
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
            Ok::<_, ProjectDuplicateError>(media_asset::ActiveModel {
                project_id: Set(new_project_id),
                root_project_id: Set(new_project_id),
                source_chapter_id: Set(None),
                source_snapshot_chapter_id: Set(None),
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
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inserted = media_asset::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != assets.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的媒体资产数量与源记录不一致".to_string(),
            operation: "复制自由视频项目媒体资产".to_string(),
        });
    }
    let mut asset_id_map = HashMap::with_capacity(assets.len());
    for (asset, copied) in assets.iter().zip(inserted) {
        asset_id_map.insert(asset.id, copied.id);
    }
    Ok(asset_id_map)
}
