use a4n_db_models::models::project::media_authoring_document as document;
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringSourceRef,
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use serde_json::Value;

use crate::project::family::import::{
    DatabaseProjectFamilyImportPackage, ImportIdMaps, ProjectFamilyImportError, map_optional,
    map_required,
};

pub(super) async fn insert_authoring_document_rows(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.authoring_documents,
        record,
        document::ActiveModel,
        maps.authoring_document_ids,
        "导入媒体创作文档失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                record.project_id,
                "创作文档所属项目",
            )?);
            active.source_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                record.source_snapshot_id,
                "创作文档基准快照",
            )?);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                record.source_chapter_id,
                "创作文档来源章节",
            )?);
        }
    );
    for receipt in &package.authoring_receipts {
        let mut active: a4n_db_models::models::project::media_authoring_migration_receipt::ActiveModel = receipt.clone().into();
        active.project_id = Set(map_required(
            &maps.project_ids,
            receipt.project_id,
            "迁移回执所属项目",
        )?);
        active
            .insert(txn)
            .await
            .map_err(ProjectFamilyImportError::Database)?;
    }
    Ok(())
}

/// IDs must be allocated before documents can refer to one another. Finalize
/// their snapshots after film resource hashes are remapped, within the import transaction.
pub(super) async fn remap_authoring_documents(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let mut ordered = package.authoring_documents.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|record| (record.kind != "adaptation_plan", record.id));
    for record in ordered {
        let expected =
            crate::generation::canonical_json_sha256(&(&record.inputs_json, &record.document_json))
                .map_err(invalid)?;
        if expected != record.document_sha256 {
            return Err(invalid(format!("创作文档 {} 内容哈希不一致", record.id)));
        }
        let mut inputs: AuthoringDocumentInputs =
            serde_json::from_value(record.inputs_json.clone()).map_err(invalid)?;
        for source in &mut inputs.sources {
            remap_source(source, maps)?;
        }
        for dependency in &mut inputs.dependencies {
            let old = dependency.id;
            dependency.id = map_required(&maps.authoring_document_ids, old, "创作文档依赖")?;
            dependency.sha256 = maps
                .authoring_document_sha256s
                .get(&old)
                .cloned()
                .ok_or_else(|| invalid("创作文档依赖必须先于下游文档导入"))?;
        }
        if let Some(reference) = inputs.video_reference.as_mut() {
            remap_source(&mut reference.source, maps)?;
            let producer = maps
                .shooting_script_sources
                .get(&reference.script_id)
                .filter(|script| Some(script.project_id) == reference.video_project_id);
            reference.video_project_id = reference
                .video_project_id
                .and_then(|id| maps.project_ids.get(&id).copied());
            if let Some(producer) = producer.filter(|_| reference.video_project_id.is_some()) {
                let old_id = reference.script_id;
                reference.script_id =
                    map_required(&maps.shooting_script_ids, old_id, "视频参考剧本")?;
                // A newer producer is not the captured version. Keep its frozen
                // hash so import cannot silently claim the reference is current.
                if producer.revision == reference.script_revision
                    && producer.sha256 == reference.script_sha256
                {
                    reference.script_sha256 = maps
                        .shooting_script_sha256s
                        .get(&old_id)
                        .cloned()
                        .ok_or_else(|| invalid("视频参考剧本缺少重映射哈希"))?;
                }
            } else {
                reference.video_project_id = None;
            }
        }
        let kind: AuthoringDocumentKind =
            serde_json::from_value(Value::String(record.kind.clone())).map_err(invalid)?;
        let mut content = record.document_json.clone();
        match kind {
            AuthoringDocumentKind::AdaptationPlan => {}
            AuthoringDocumentKind::MangaPagePlan => {
                let mut plan: a4n_domain::manga::MangaPagePlanV4 =
                    serde_json::from_value(content).map_err(invalid)?;
                plan.adaptation_plan_id = map_required(
                    &maps.authoring_document_ids,
                    plan.adaptation_plan_id,
                    "漫画分页改编计划",
                )?;
                content = serde_json::to_value(plan).map_err(invalid)?;
            }
            AuthoringDocumentKind::GalgameBranchPlan => remap_branch_chapters(&mut content, maps)?,
        }
        let inputs = serde_json::to_value(inputs).map_err(invalid)?;
        let hash =
            crate::generation::canonical_json_sha256(&(&inputs, &content)).map_err(invalid)?;
        document::ActiveModel {
            id: Set(map_required(
                &maps.authoring_document_ids,
                record.id,
                "媒体创作文档",
            )?),
            inputs_json: Set(inputs),
            document_json: Set(content),
            document_sha256: Set(hash.clone()),
            ..Default::default()
        }
        .update(txn)
        .await
        .map_err(ProjectFamilyImportError::Database)?;
        maps.authoring_document_sha256s.insert(record.id, hash);
    }
    Ok(())
}

fn remap_source(
    source: &mut AuthoringSourceRef,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    source.root_project_id =
        map_required(&maps.project_ids, source.root_project_id, "改编原作项目")?;
    source.source_chapter_id =
        map_required(&maps.chapter_ids, source.source_chapter_id, "改编原文章节")?;
    source.source_snapshot_id = map_required(
        &maps.source_snapshot_ids,
        source.source_snapshot_id,
        "改编原文快照",
    )?;
    source.source_snapshot_chapter_id = map_required(
        &maps.source_snapshot_chapter_ids,
        source.source_snapshot_chapter_id,
        "改编原文快照章节",
    )?;
    Ok(())
}

fn remap_branch_chapters(
    content: &mut Value,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let choices = content
        .get_mut("choices")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("Galgame 分支计划缺少 choices"))?;
    for choice in choices {
        remap_unit_ref(choice.get_mut("after_beat_ref"), maps)?;
        let options = choice
            .get_mut("options")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| invalid("Galgame 分支缺少 options"))?;
        for option in options {
            remap_unit_ref(option.get_mut("returns_to"), maps)?;
        }
    }
    Ok(())
}

fn remap_unit_ref(
    value: Option<&mut Value>,
    maps: &ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let object = value
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid("Galgame 分支单元引用缺失"))?;
    let chapter_id = object
        .get("source_chapter_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| invalid("Galgame 分支单元章节身份缺失"))?;
    object.insert(
        "source_chapter_id".into(),
        map_required(&maps.chapter_ids, chapter_id, "分支原文章节")?.into(),
    );
    Ok(())
}

fn invalid(error: impl std::fmt::Display) -> ProjectFamilyImportError {
    ProjectFamilyImportError::invalid_package(error.to_string())
}
