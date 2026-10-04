use super::*;

macro_rules! insert_import_row {
    ($txn:expr, $record:expr, $active_ty:path, $err:literal, |$active:ident| $prepare:block) => {{
        let mut $active: $active_ty = ($record).clone().into();
        $active.id = sea_orm::NotSet;
        $prepare
        $active
            .insert($txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: $err,
                source,
            })?
    }};
}

macro_rules! insert_mapped_import_row {
    ($txn:expr, $record:expr, $active_ty:path, $map:expr, $err:literal, |$active:ident| $prepare:block) => {{
        let inserted = insert_import_row!($txn, $record, $active_ty, $err, |$active| $prepare);
        ($map).insert(($record).id, inserted.id);
    }};
}

macro_rules! insert_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $err:literal, |$active:ident| $prepare:block) => {{
        let mut rows = Vec::new();
        for $record in $records {
            let mut $active: $active_ty = ($record).clone().into();
            $active.id = sea_orm::NotSet;
            $prepare
            rows.push($active);
        }
        if !rows.is_empty() {
            <<$active_ty as ActiveModelTrait>::Entity as sea_orm::EntityTrait>::insert_many(rows)
                .exec_without_returning($txn)
                .await
                .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                    context: $err,
                    source,
                })?;
        }
    }};
}

macro_rules! insert_mapped_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $map:expr, $err:literal, |$active:ident| $prepare:block) => {{
        let mut rows = Vec::new();
        let mut source_ids = Vec::new();
        for $record in $records {
            let mut $active: $active_ty = ($record).clone().into();
            $active.id = sea_orm::NotSet;
            $prepare
            rows.push($active);
            source_ids.push(($record).id);
        }
        if !rows.is_empty() {
            let inserted =
                <<$active_ty as ActiveModelTrait>::Entity as sea_orm::EntityTrait>::insert_many(
                    rows,
                )
                .exec_with_returning_many($txn)
                .await
                .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                    context: $err,
                    source,
                })?;
            if inserted.len() != source_ids.len() {
                return Err(ProjectFamilyImportError::invalid_package(format!(
                    "批量导入返回行数不一致: inserted={}, expected={}",
                    inserted.len(),
                    source_ids.len()
                )));
            }
            for (source_id, inserted) in source_ids.into_iter().zip(inserted) {
                ($map).insert(source_id, inserted.id);
            }
        }
    }};
}

macro_rules! insert_project_scoped_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $err:literal, $project_id:expr) => {{
        insert_import_rows!($txn, $records, $record, $active_ty, $err, |active| {
            active.project_id = Set($project_id);
        });
    }};
}

macro_rules! insert_project_scoped_mapped_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $map:expr, $err:literal, $project_id:expr) => {{
        insert_mapped_import_rows!($txn, $records, $record, $active_ty, $map, $err, |active| {
            active.project_id = Set($project_id);
        });
    }};
}

pub(super) async fn import_package_rows<Importer>(
    user_id: i64,
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    media_importer: &mut Importer,
) -> Result<(Vec<novel_project::Model>, ImportIdMaps), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let mut maps = ImportIdMaps::default();
    let root_project = insert_root_project(txn, user_id, package, &mut maps).await?;
    insert_text_core(txn, package, &mut maps).await?;
    insert_source_snapshots(txn, package, &mut maps).await?;
    let mut imported_projects = vec![root_project];
    imported_projects.extend(insert_derived_projects(txn, user_id, package, &mut maps).await?);
    insert_scene_profiles(txn, package, &mut maps).await?;
    insert_text_anchor_rows(txn, package, &mut maps).await?;
    insert_media_project_rows(user_id, txn, package, &mut maps, media_importer).await?;
    Ok((imported_projects, maps))
}

pub(super) async fn insert_media_project_rows<Importer: ProjectFamilyMediaFileImporter>(
    user_id: i64,
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
    media_importer: &mut Importer,
) -> Result<(), ProjectFamilyImportError> {
    insert_visual_plan_rows(user_id, txn, package, maps).await?;
    authoring::insert_authoring_document_rows(txn, package, maps).await?;
    insert_galgame_rows(txn, package, maps).await?;
    insert_manga_rows(txn, package, maps, media_importer).await?;
    insert_video_generation_rows(txn, package, maps, media_importer).await?;
    insert_generation_rows_before_media(txn, package, maps).await?;
    insert_media_rows(txn, package, maps, media_importer).await?;
    insert_video_segments(txn, package, maps).await?;
    insert_assembly_rows(txn, package, maps).await?;
    update_generation_spec_deferred_targets(txn, package, maps).await?;
    update_video_generation_refs(txn, package, maps).await?;
    insert_media_provenance_rows(txn, package, maps).await?;
    insert_import_rows!(
        txn,
        &package.media_bindings,
        binding,
        media_binding::ActiveModel,
        "导入媒体绑定失败",
        |active| {
            active.media_asset_id = Set(map_required(
                &maps.media_asset_ids,
                binding.media_asset_id,
                "媒体绑定 media_asset_id",
            )?);
            active.target_id = Set(remap_media_binding_target_id(binding, maps)?);
        }
    );
    insert_source_sync_rows(txn, package, maps).await?;
    Ok(())
}

mod authoring;
mod root_text;
pub(super) use root_text::insert_scene_profiles;
use root_text::*;

mod snapshots;
use snapshots::*;

mod visual_plan;
use visual_plan::insert_visual_plan_rows;

mod galgame;
use galgame::*;

mod generation_media;
use generation_media::{
    insert_generation_rows_before_media, insert_media_provenance_rows, insert_media_rows,
};

mod manga;
use manga::insert_manga_rows;

mod video;
use video::*;

mod source_sync;
use source_sync::*;
