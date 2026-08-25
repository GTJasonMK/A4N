use a4n_core::models::canvas::canvas_text_asset;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
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
        let source = crate::project::create_freeform_video_project_record(
            &db,
            1,
            crate::project::CreateFreeformVideoProjectParams {
                title: "复制文本资产源".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let target = crate::project::create_freeform_video_project_record(
            &db,
            1,
            crate::project::CreateFreeformVideoProjectParams {
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
        for source_row in source_rows {
            assert!(id_map.contains_key(&source_row.id));
        }
    }
}
