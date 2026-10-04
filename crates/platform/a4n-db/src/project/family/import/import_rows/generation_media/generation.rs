use a4n_db_models::models::generation::{
    generation_spec, generation_spec_entity_ref, generation_spec_text_anchor, prompt_artifact,
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};

use crate::project::family::import::{
    DatabaseProjectFamilyImportPackage, ImportIdMaps, ProjectFamilyImportError, map_optional,
    map_required, remap_generation_spec_params_json, remap_generation_spec_source_version_hash,
    remap_generation_target_id, remap_profile_id,
};

pub(in crate::project::family::import::import_rows) async fn insert_generation_rows_before_media(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        package
            .generation_specs
            .iter()
            .filter(|spec| spec.source_context_kind.as_deref()
                != Some(generation_spec::SOURCE_CONTEXT_FREEFORM)),
        spec,
        generation_spec::ActiveModel,
        maps.generation_spec_ids,
        "导入生成意图失败",
        |active| {
            let source_snapshot_id = spec.source_snapshot_id.ok_or_else(|| {
                ProjectFamilyImportError::InvalidPackage(format!(
                    "项目族导入生成意图缺少 source_snapshot_id: {}",
                    spec.id
                ))
            })?;
            active.project_id = Set(map_required(
                &maps.project_ids,
                spec.project_id,
                "生成意图 project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                spec.root_project_id,
                "生成意图 root_project_id",
            )?);
            active.source_context_kind = Set(Some("snapshot_backed".to_string()));
            active.source_snapshot_id = Set(Some(map_required(
                &maps.source_snapshot_ids,
                source_snapshot_id,
                "生成意图 source_snapshot_id",
            )?));
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                spec.source_chapter_id,
                "生成意图 source_chapter_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                spec.source_snapshot_chapter_id,
                "生成意图 source_snapshot_chapter_id",
            )?);
            active.source_version_hash =
                Set(remap_generation_spec_source_version_hash(spec, maps)?);
            active.target_entity_id = Set(remap_generation_target_id(spec, maps, false)?);
            active.params_json = Set(remap_generation_spec_params_json(spec, maps)?);
        }
    );
    let anchor_ref_inserts = package
        .generation_spec_text_anchors
        .iter()
        .map(|record| {
            Ok(generation_spec_text_anchor::ActiveModel {
                generation_spec_id: Set(map_required(
                    &maps.generation_spec_ids,
                    record.generation_spec_id,
                    "生成意图锚点 generation_spec_id",
                )?),
                text_anchor_id: Set(map_required(
                    &maps.text_anchor_ids,
                    record.text_anchor_id,
                    "生成意图锚点 text_anchor_id",
                )?),
                role: Set(record.role.clone()),
            })
        })
        .collect::<Result<Vec<_>, ProjectFamilyImportError>>()?;
    if !anchor_ref_inserts.is_empty() {
        generation_spec_text_anchor::Entity::insert_many(anchor_ref_inserts)
            .exec_without_returning(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入生成意图锚点引用失败",
                source,
            })?;
    }
    let entity_ref_inserts = package
        .generation_spec_entity_refs
        .iter()
        .map(|record| {
            Ok(generation_spec_entity_ref::ActiveModel {
                generation_spec_id: Set(map_required(
                    &maps.generation_spec_ids,
                    record.generation_spec_id,
                    "生成意图实体 generation_spec_id",
                )?),
                entity_kind: Set(record.entity_kind.clone()),
                entity_id: Set(remap_profile_id(
                    &record.entity_kind,
                    record.entity_id,
                    maps,
                    "生成意图实体 entity_id",
                )?),
                role: Set(record.role.clone()),
            })
        })
        .collect::<Result<Vec<_>, ProjectFamilyImportError>>()?;
    if !entity_ref_inserts.is_empty() {
        generation_spec_entity_ref::Entity::insert_many(entity_ref_inserts)
            .exec_without_returning(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入生成意图实体引用失败",
                source,
            })?;
    }
    insert_mapped_import_rows!(
        txn,
        &package.prompt_artifacts,
        artifact,
        prompt_artifact::ActiveModel,
        maps.prompt_artifact_ids,
        "导入提示词失败",
        |active| {
            active.generation_spec_id = Set(map_required(
                &maps.generation_spec_ids,
                artifact.generation_spec_id,
                "提示词 generation_spec_id",
            )?);
        }
    );
    Ok(())
}
