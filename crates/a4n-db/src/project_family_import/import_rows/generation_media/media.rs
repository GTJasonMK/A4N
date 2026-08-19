use a4n_core::models::media::media_asset::{FILE_PATH_FIELD, THUMBNAIL_PATH_FIELD};
use a4n_core::models::{
    generation::generation_spec_media_ref,
    media::{media_asset, media_provenance},
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage,
    ProjectFamilyMediaFileImporter, map_optional, map_required,
    media_asset_status_for_import_insert, remap_media_request_snapshot,
};

pub(in crate::project_family_import::import_rows) async fn insert_media_rows<Importer>(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    for asset in &package.media_assets {
        let original_status = asset.status.clone();
        let inserted = insert_import_row!(
            txn,
            asset,
            media_asset::ActiveModel,
            "导入媒体资产失败",
            |active| {
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
            }
        );
        maps.media_asset_ids.insert(asset.id, inserted.id);

        let file_path =
            media_importer.write_imported_media_field(asset, &inserted, FILE_PATH_FIELD)?;
        let thumbnail_path =
            media_importer.write_imported_media_field(asset, &inserted, THUMBNAIL_PATH_FIELD)?;
        let mut update: media_asset::ActiveModel = inserted.into();
        update.file_path = Set(file_path);
        update.thumbnail_path = Set(thumbnail_path);
        update.file_size = Set(asset.file_size);
        update.checksum = Set(asset.checksum.clone());
        update.status = Set(original_status);
        update
            .update(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新媒体资产文件路径失败",
                source,
            })?;
    }
    for record in &package.generation_spec_media_refs {
        generation_spec_media_ref::ActiveModel {
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
        }
        .insert(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "导入生成意图媒体引用失败",
            source,
        })?;
    }
    for provenance in &package.media_provenance {
        media_provenance::ActiveModel {
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
            provider_job_id: Set(provenance.provider_job_id.clone()),
            source_media_asset_id: Set(map_optional(
                &maps.media_asset_ids,
                provenance.source_media_asset_id,
                "媒体来源 source_media_asset_id",
            )?),
            source_external_path: Set(provenance.source_external_path.clone()),
            imported_from_user: Set(provenance.imported_from_user),
            request_snapshot_json: Set(remap_media_request_snapshot(
                provenance.request_snapshot_json.as_ref(),
                maps,
            )?),
            created_at: Set(provenance.created_at),
        }
        .insert(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "导入媒体来源失败",
            source,
        })?;
    }
    Ok(())
}
