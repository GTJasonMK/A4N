use a4n_db_models::models::canvas::{canvas_text_asset, canvas_text_asset_version};
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
    if assets.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = assets
        .iter()
        .map(|asset| canvas_text_asset::ActiveModel {
            project_id: Set(new_project_id),
            title: Set(asset.title.clone()),
            text_kind: Set(asset.text_kind.clone()),
            content: Set(asset.content.clone()),
            content_hash: Set(asset.content_hash.clone()),
            current_version: Set(asset.current_version),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    let inserted = canvas_text_asset::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if inserted.len() != assets.len() {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的画布文本资产数量与源记录不一致".to_string(),
            operation: "复制自由视频项目画布文本资产".to_string(),
        });
    }
    let mut id_map = HashMap::new();
    for (asset, row) in assets.iter().zip(inserted) {
        id_map.insert(asset.id, row.id);
        let mut versions = canvas_text_asset_version::Entity::find()
            .filter(canvas_text_asset_version::Column::CanvasTextAssetId.eq(asset.id))
            .order_by_asc(canvas_text_asset_version::Column::Version)
            .all(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        if versions.is_empty() {
            versions.push(canvas_text_asset_version::Model {
                id: 0,
                canvas_text_asset_id: asset.id,
                version: asset.current_version,
                content: asset.content.clone(),
                content_hash: asset.content_hash.clone(),
                created_at: asset.created_at,
            });
        }
        for version in versions {
            canvas_text_asset_version::ActiveModel {
                canvas_text_asset_id: Set(row.id),
                version: Set(version.version),
                content: Set(version.content),
                content_hash: Set(version.content_hash),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(ProjectDuplicateError::Database)?;
        }
    }
    Ok(id_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{EntityTrait, QueryFilter};

    #[tokio::test]
    async fn copies_canvas_text_assets_in_one_batch() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let source = crate::project::create_canvas_project_record(
            &db,
            1,
            crate::project::CreateCanvasProjectParams {
                title: "复制文本资产源".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let target = crate::project::create_canvas_project_record(
            &db,
            1,
            crate::project::CreateCanvasProjectParams {
                title: "复制文本资产目标".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let assets = (0..20)
            .map(|index| canvas_text_asset::ActiveModel {
                project_id: Set(source.id),
                title: Set(format!("资产 {index}")),
                text_kind: Set("prompt".to_string()),
                content: Set(format!("内容 {index}")),
                content_hash: Set(format!("hash-{index}")),
                current_version: Set(1),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let source_rows = canvas_text_asset::Entity::insert_many(assets)
            .exec_with_returning_many(&db)
            .await
            .unwrap();

        let id_map = copy_freeform_canvas_text_assets_record(&db, source.id, target.id, now)
            .await
            .unwrap();
        assert_eq!(id_map.len(), source_rows.len());
        let copied = canvas_text_asset::Entity::find()
            .filter(canvas_text_asset::Column::ProjectId.eq(target.id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(copied.len(), source_rows.len());
        let copied_by_id = copied
            .iter()
            .map(|asset| (asset.id, asset))
            .collect::<std::collections::HashMap<_, _>>();
        for source_row in source_rows {
            let copied_id = id_map[&source_row.id];
            let copied_row = copied_by_id[&copied_id];
            assert_ne!(copied_row.id, source_row.id);
            assert_eq!(copied_row.project_id, target.id);
            assert_eq!(copied_row.title, source_row.title);
            assert_eq!(copied_row.text_kind, source_row.text_kind);
            assert_eq!(copied_row.content, source_row.content);
            assert_eq!(copied_row.content_hash, source_row.content_hash);
        }
    }
}
