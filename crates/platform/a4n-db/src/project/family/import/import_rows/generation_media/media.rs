use a4n_db_models::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_db_models::models::{
    generation::generation_spec_media_ref,
    media::{media_asset, media_provenance},
};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};

use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage,
    ProjectFamilyMediaFileImporter, map_optional, map_required,
    media_asset_status_for_import_insert, remap_media_request_snapshot,
    remap_media_source_external_path,
};

pub(in crate::project::family::import::import_rows) async fn insert_media_rows<Importer>(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    if !package.media_assets.is_empty() {
        let rows = package
            .media_assets
            .iter()
            .map(|asset| {
                let original_status = asset.status.clone();
                let mut active: media_asset::ActiveModel = asset.clone().into();
                active.id = sea_orm::NotSet;
                active.project_id = Set(map_required(
                    &maps.project_ids,
                    asset.project_id,
                    "媒体资产 project_id",
                )?);
                active.root_project_id = Set(map_required(
                    &maps.project_ids,
                    asset.root_project_id,
                    "媒体资产 root_project_id",
                )?);
                active.source_snapshot_id = Set(map_optional(
                    &maps.source_snapshot_ids,
                    asset.source_snapshot_id,
                    "媒体资产 source_snapshot_id",
                )?);
                active.source_chapter_id = Set(map_optional(
                    &maps.chapter_ids,
                    asset.source_chapter_id,
                    "媒体资产 source_chapter_id",
                )?);
                active.source_snapshot_chapter_id = Set(map_optional(
                    &maps.source_snapshot_chapter_ids,
                    asset.source_snapshot_chapter_id,
                    "媒体资产 source_snapshot_chapter_id",
                )?);
                active.file_path = Set(None);
                active.thumbnail_path = Set(None);
                active.file_size = Set(None);
                active.checksum = Set(None);
                active.status =
                    Set(media_asset_status_for_import_insert(&original_status).to_string());
                Ok::<_, ProjectFamilyImportError>((active, original_status))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let (rows, original_statuses): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
        let inserted = media_asset::Entity::insert_many(rows)
            .exec_with_returning_many(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入媒体资产失败",
                source,
            })?;
        if inserted.len() != package.media_assets.len() {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "导入媒体资产批量写入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                package.media_assets.len()
            )));
        }
        let mut file_path_case = CaseStatement::new();
        let mut thumbnail_path_case = CaseStatement::new();
        let mut file_size_case = CaseStatement::new();
        let mut checksum_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut asset_ids = Vec::with_capacity(inserted.len());
        for ((asset, original_status), inserted) in package
            .media_assets
            .iter()
            .zip(original_statuses)
            .zip(inserted)
        {
            let file_path = media_importer.write_imported_media_field(
                inserted.project_id,
                CanvasNodeRefKind::MediaAsset.as_str(),
                asset.id,
                inserted.id,
                FILE_PATH_FIELD,
            )?;
            let thumbnail_path = media_importer.write_imported_media_field(
                inserted.project_id,
                CanvasNodeRefKind::MediaAsset.as_str(),
                asset.id,
                inserted.id,
                THUMBNAIL_PATH_FIELD,
            )?;
            if original_status == "completed"
                && file_path
                    .as_deref()
                    .is_none_or(|path| path.trim().is_empty())
            {
                return Err(ProjectFamilyImportError::invalid_package(format!(
                    "已完成媒体资产 {} 缺少文件",
                    asset.id
                )));
            }
            file_path_case = file_path_case.case(
                media_asset::Column::Id.eq(inserted.id),
                Expr::value(file_path),
            );
            thumbnail_path_case = thumbnail_path_case.case(
                media_asset::Column::Id.eq(inserted.id),
                Expr::value(thumbnail_path),
            );
            file_size_case = file_size_case.case(
                media_asset::Column::Id.eq(inserted.id),
                Expr::value(asset.file_size),
            );
            checksum_case = checksum_case.case(
                media_asset::Column::Id.eq(inserted.id),
                Expr::value(asset.checksum.clone()),
            );
            status_case = status_case.case(
                media_asset::Column::Id.eq(inserted.id),
                Expr::value(original_status),
            );
            asset_ids.push(inserted.id);
            maps.media_asset_ids.insert(asset.id, inserted.id);
        }
        let updated = media_asset::Entity::update_many()
            .col_expr(
                media_asset::Column::FilePath,
                file_path_case
                    .finally(Expr::col(media_asset::Column::FilePath))
                    .into(),
            )
            .col_expr(
                media_asset::Column::ThumbnailPath,
                thumbnail_path_case
                    .finally(Expr::col(media_asset::Column::ThumbnailPath))
                    .into(),
            )
            .col_expr(
                media_asset::Column::FileSize,
                file_size_case
                    .finally(Expr::col(media_asset::Column::FileSize))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Checksum,
                checksum_case
                    .finally(Expr::col(media_asset::Column::Checksum))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Status,
                status_case
                    .finally(Expr::col(media_asset::Column::Status))
                    .into(),
            )
            .filter(media_asset::Column::Id.is_in(asset_ids.clone()))
            .exec(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新媒体资产文件路径失败",
                source,
            })?;
        if updated.rows_affected != asset_ids.len() as u64 {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "更新媒体资产文件路径数量不一致: updated={}, expected={}",
                updated.rows_affected,
                asset_ids.len()
            )));
        }
    }
    let media_ref_inserts = package
        .generation_spec_media_refs
        .iter()
        .map(|record| {
            Ok(generation_spec_media_ref::ActiveModel {
                generation_spec_id: Set(map_required(
                    &maps.generation_spec_ids,
                    record.generation_spec_id,
                    "生成意图媒体 generation_spec_id",
                )?),
                media_asset_id: Set(map_required(
                    &maps.media_asset_ids,
                    record.media_asset_id,
                    "生成意图媒体 media_asset_id",
                )?),
                role: Set(record.role.clone()),
            })
        })
        .collect::<Result<Vec<_>, ProjectFamilyImportError>>()?;
    if !media_ref_inserts.is_empty() {
        generation_spec_media_ref::Entity::insert_many(media_ref_inserts)
            .exec_without_returning(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入生成意图媒体引用失败",
                source,
            })?;
    }
    Ok(())
}

pub(in crate::project::family::import::import_rows) async fn insert_media_provenance_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let inserts = package
        .media_provenance
        .iter()
        .map(|provenance| {
            Ok(media_provenance::ActiveModel {
                media_asset_id: Set(map_required(
                    &maps.media_asset_ids,
                    provenance.media_asset_id,
                    "媒体来源 media_asset_id",
                )?),
                generation_spec_id: Set(map_optional(
                    &maps.generation_spec_ids,
                    provenance.generation_spec_id,
                    "媒体来源 generation_spec_id",
                )?),
                prompt_artifact_id: Set(map_optional(
                    &maps.prompt_artifact_ids,
                    provenance.prompt_artifact_id,
                    "媒体来源 prompt_artifact_id",
                )?),
                provider_job_id: Set(None),
                source_media_asset_id: Set(map_optional(
                    &maps.media_asset_ids,
                    provenance.source_media_asset_id,
                    "媒体来源 source_media_asset_id",
                )?),
                source_external_path: Set(remap_media_source_external_path(
                    provenance.source_external_path.as_deref(),
                    maps,
                )?),
                imported_from_user: Set(provenance.imported_from_user),
                request_snapshot_json: Set(remap_media_request_snapshot(
                    provenance.request_snapshot_json.as_ref(),
                    maps,
                )?),
                created_at: Set(provenance.created_at),
            })
        })
        .collect::<Result<Vec<_>, ProjectFamilyImportError>>()?;
    if !inserts.is_empty() {
        media_provenance::Entity::insert_many(inserts)
            .exec_without_returning(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入媒体来源失败",
                source,
            })?;
    }
    Ok(())
}
