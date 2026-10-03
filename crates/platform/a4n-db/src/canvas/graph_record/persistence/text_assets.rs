use a4n_db_models::models::canvas::{canvas_text_asset, canvas_text_asset_version};
use a4n_domain::canvas::{
    CanvasTextAssetRecord, CanvasTextAssetRecordInput, CanvasTextAssetVersionRecord,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use sha2::{Digest, Sha256};

use super::super::records::canvas_text_asset_record;
use super::super::validation::required_canvas_record_text;
use super::context::require_canvas_text_asset_project_record;
use crate::canvas::CanvasDbError;

pub async fn upsert_canvas_text_asset_record(
    db: &DatabaseConnection,
    project_id: i64,
    input: CanvasTextAssetRecordInput,
    now: chrono::NaiveDateTime,
) -> Result<CanvasTextAssetRecord, CanvasDbError> {
    let title = required_canvas_record_text(&input.title, "title")?;
    let text_kind = normalize_canvas_record_text_kind(&input.text_kind)?;
    let content = input.content.trim().to_string();
    if content.is_empty() {
        return Err(CanvasDbError::InvalidArgument(
            "content 不能为空".to_string(),
        ));
    }
    let content_hash = canvas_record_sha256_hex(content.as_bytes());
    let txn = db.begin().await.map_err(CanvasDbError::Database)?;
    require_canvas_text_asset_project_record(&txn, project_id).await?;
    match input.id {
        Some(id) => {
            let existing = canvas_text_asset::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| CanvasDbError::NotFound(format!("画布文本资产不存在: {id}")))?;
            if existing.project_id != project_id {
                return Err(CanvasDbError::Forbidden(
                    "文本资产不属于当前项目".to_string(),
                ));
            }
            let expected_version = input.expected_version.unwrap_or(existing.current_version);
            if expected_version <= 0 {
                return Err(CanvasDbError::InvalidArgument(
                    "expected_version 必须大于 0".to_string(),
                ));
            }
            if existing.current_version != expected_version {
                return Err(CanvasDbError::TextAssetVersionConflict {
                    canvas_text_asset_id: id,
                    expected_version,
                    current_version: existing.current_version,
                });
            }
            let next_version = expected_version.checked_add(1).ok_or_else(|| {
                CanvasDbError::InvalidArgument("文本素材版本超出 i32 范围".to_string())
            })?;
            let update = canvas_text_asset::Entity::update_many()
                .col_expr(
                    canvas_text_asset::Column::Title,
                    sea_orm::sea_query::Expr::value(title),
                )
                .col_expr(
                    canvas_text_asset::Column::TextKind,
                    sea_orm::sea_query::Expr::value(text_kind.to_string()),
                )
                .col_expr(
                    canvas_text_asset::Column::Content,
                    sea_orm::sea_query::Expr::value(content.clone()),
                )
                .col_expr(
                    canvas_text_asset::Column::ContentHash,
                    sea_orm::sea_query::Expr::value(content_hash.clone()),
                )
                .col_expr(
                    canvas_text_asset::Column::CurrentVersion,
                    sea_orm::sea_query::Expr::value(next_version),
                )
                .col_expr(
                    canvas_text_asset::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::value(now),
                )
                .filter(canvas_text_asset::Column::Id.eq(id))
                .filter(canvas_text_asset::Column::ProjectId.eq(project_id))
                .filter(canvas_text_asset::Column::CurrentVersion.eq(expected_version))
                .exec(&txn)
                .await?;
            if update.rows_affected != 1 {
                let current_version = canvas_text_asset::Entity::find_by_id(id)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("画布文本资产不存在: {id}")))?
                    .current_version;
                return Err(CanvasDbError::TextAssetVersionConflict {
                    canvas_text_asset_id: id,
                    expected_version,
                    current_version,
                });
            }
            canvas_text_asset_version::ActiveModel {
                canvas_text_asset_id: Set(id),
                version: Set(next_version),
                content: Set(content),
                content_hash: Set(content_hash),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            let record = canvas_text_asset::Entity::find_by_id(id)
                .one(&txn)
                .await?
                .ok_or_else(|| CanvasDbError::NotFound(format!("画布文本资产不存在: {id}")))?;
            let record = canvas_text_asset_record(record);
            txn.commit().await.map_err(CanvasDbError::Database)?;
            Ok(record)
        }
        None => {
            if input.expected_version.is_some() {
                return Err(CanvasDbError::InvalidArgument(
                    "新建文本素材不能提供 expected_version".to_string(),
                ));
            }
            let asset = canvas_text_asset::ActiveModel {
                project_id: Set(project_id),
                title: Set(title),
                text_kind: Set(text_kind.to_string()),
                content: Set(content.clone()),
                content_hash: Set(content_hash.clone()),
                current_version: Set(1),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            canvas_text_asset_version::ActiveModel {
                canvas_text_asset_id: Set(asset.id),
                version: Set(1),
                content: Set(content),
                content_hash: Set(content_hash),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            let record = canvas_text_asset_record(asset);
            txn.commit().await.map_err(CanvasDbError::Database)?;
            Ok(record)
        }
    }
}

pub async fn canvas_text_assets_by_project_record(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<CanvasTextAssetRecord>, CanvasDbError> {
    require_canvas_text_asset_project_record(db, project_id).await?;
    Ok(canvas_text_asset::Entity::find()
        .filter(canvas_text_asset::Column::ProjectId.eq(project_id))
        .order_by_asc(canvas_text_asset::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(canvas_text_asset_record)
        .collect())
}

pub async fn canvas_text_asset_versions_record(
    db: &DatabaseConnection,
    project_id: i64,
    asset_id: i64,
) -> Result<Vec<CanvasTextAssetVersionRecord>, CanvasDbError> {
    require_canvas_text_asset_project_record(db, project_id).await?;
    let asset = canvas_text_asset::Entity::find_by_id(asset_id)
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("画布文本资产不存在: {asset_id}")))?;
    if asset.project_id != project_id {
        return Err(CanvasDbError::Forbidden(
            "文本素材不属于当前项目".to_string(),
        ));
    }
    Ok(canvas_text_asset_version::Entity::find()
        .filter(canvas_text_asset_version::Column::CanvasTextAssetId.eq(asset_id))
        .order_by_desc(canvas_text_asset_version::Column::Version)
        .all(db)
        .await?
        .into_iter()
        .map(|row| CanvasTextAssetVersionRecord {
            canvas_text_asset_id: row.canvas_text_asset_id,
            version: row.version,
            content: row.content,
            content_hash: row.content_hash,
            created_at: row.created_at,
        })
        .collect())
}

fn normalize_canvas_record_text_kind(value: &str) -> Result<&'static str, CanvasDbError> {
    match value.trim() {
        "script" => Ok("script"),
        "shot_note" => Ok("shot_note"),
        "dialogue" => Ok("dialogue"),
        "narration" => Ok("narration"),
        "character_note" => Ok("character_note"),
        "freeform" => Ok("freeform"),
        "canvas_output" => Ok("canvas_output"),
        other => Err(CanvasDbError::InvalidArgument(format!(
            "未知画布文本类型: {other}"
        ))),
    }
}

fn canvas_record_sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::normalize_canvas_record_text_kind;

    #[test]
    fn canvas_output_is_a_canonical_canvas_text_kind() {
        assert_eq!(
            normalize_canvas_record_text_kind(" canvas_output ").unwrap(),
            "canvas_output"
        );
    }

    #[test]
    fn unknown_canvas_text_kind_is_rejected() {
        let error =
            normalize_canvas_record_text_kind("legacy_output").expect_err("未知文本类型必须失败");
        assert!(error.to_string().contains("未知画布文本类型"));
    }
}
