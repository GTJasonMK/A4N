use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_required,
};
use a4n_db_models::models::{
    chapter::chapter,
    character::{
        attribute_event, character_state_index, foreshadowing_index, protagonist_behavior_record,
        protagonist_deletion_mark, snapshot_reference,
    },
    project::{material_fact_change, project_fact_feedback},
};
use a4n_domain::fact_feedback::FactSourceRevision;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};
use serde_json::Value;

pub(super) async fn insert_feedback_rows(
    tx: &DatabaseTransaction,
    project_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let Some(archive) = &package.text_core.fact_feedback else {
        return Ok(());
    };
    for ids in [
        archive
            .foreshadowing
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        archive.attribute_events.iter().map(|row| row.id).collect(),
        archive.snapshots.iter().map(|row| row.id).collect(),
        archive.state_indexes.iter().map(|row| row.id).collect(),
        archive.behaviors.iter().map(|row| row.id).collect(),
        archive.deletion_marks.iter().map(|row| row.id).collect(),
    ] {
        if ids.iter().any(|id| *id <= 0)
            || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
        {
            return Err(ProjectFamilyImportError::invalid_package(
                "事实归档含无效或重复记录 ID",
            ));
        }
    }
    for row in &archive.foreshadowing {
        if row.project_id != package.root_project_id
            || a4n_domain::character::ForeshadowingStatus::parse(&row.status).is_none()
        {
            return Err(ProjectFamilyImportError::invalid_package(
                "伏笔来源项目或状态无效",
            ));
        }
        insert_mapped_import_row!(
            tx,
            row,
            foreshadowing_index::ActiveModel,
            maps.foreshadowing_ids,
            "导入伏笔事实失败",
            |active| {
                active.project_id = Set(project_id);
            }
        );
    }
    let mut event_ids = std::collections::HashMap::new();
    insert_mapped_import_rows!(
        tx,
        &archive.attribute_events,
        row,
        attribute_event::ActiveModel,
        event_ids,
        "导入角色事件失败",
        |active| {
            if a4n_domain::character::CharacterTimelineSourceKind::parse(&row.source_kind).is_none()
            {
                return Err(ProjectFamilyImportError::invalid_package(
                    "角色事件来源类型无效",
                ));
            }
            active.profile_id = Set(map_required(
                &maps.character_profile_ids,
                row.profile_id,
                "角色事件",
            )?);
        }
    );
    insert_import_rows!(
        tx,
        &archive.snapshots,
        row,
        snapshot_reference::ActiveModel,
        "导入角色快照失败",
        |active| {
            if a4n_domain::source_text_hash(&row.data.to_string()) != row.snapshot_hash {
                return Err(ProjectFamilyImportError::invalid_package(
                    "角色快照内容哈希不匹配",
                ));
            }
            let profile_id = map_required(&maps.character_profile_ids, row.profile_id, "角色快照")?;
            let mut data = row.data.clone();
            let snapshot = data
                .as_object_mut()
                .ok_or_else(|| ProjectFamilyImportError::invalid_package("角色快照必须是对象"))?;
            snapshot.insert("profile_id".into(), profile_id.into());
            snapshot.insert("project_id".into(), project_id.into());
            if let Some(changes) = snapshot.get_mut("changes").and_then(Value::as_array_mut) {
                for change in changes {
                    if let Some(id) = change.get("id").and_then(Value::as_i64) {
                        change["id"] = map_required(&event_ids, id, "角色快照属性事件")?.into();
                    }
                }
            }
            active.profile_id = Set(profile_id);
            active.snapshot_hash = Set(a4n_domain::source_text_hash(&data.to_string()));
            active.data = Set(data);
        }
    );
    insert_import_rows!(
        tx,
        &archive.state_indexes,
        row,
        character_state_index::ActiveModel,
        "导入角色状态失败",
        |active| {
            active.profile_id = Set(map_required(
                &maps.character_profile_ids,
                row.profile_id,
                "角色状态",
            )?);
        }
    );
    insert_import_rows!(
        tx,
        &archive.behaviors,
        row,
        protagonist_behavior_record::ActiveModel,
        "导入角色行为失败",
        |active| {
            active.profile_id = Set(map_required(
                &maps.character_profile_ids,
                row.profile_id,
                "角色行为",
            )?);
        }
    );
    insert_import_rows!(
        tx,
        &archive.deletion_marks,
        row,
        protagonist_deletion_mark::ActiveModel,
        "导入属性撤回记录失败",
        |active| {
            active.profile_id = Set(map_required(
                &maps.character_profile_ids,
                row.profile_id,
                "属性撤回记录",
            )?);
        }
    );
    let mut current_sources = std::collections::HashMap::new();
    for original in &package.text_core.chapters {
        let id = map_required(&maps.chapter_ids, original.id, "事实来源章节")?;
        let current = chapter::Entity::find_by_id(id)
            .one(tx)
            .await
            .map_err(db_error)?
            .ok_or_else(|| ProjectFamilyImportError::invalid_package("导入后的章节丢失"))?;
        let mut analysis = current.analysis_data.clone();
        if let Some(analysis) = analysis.as_mut() {
            let analysis = analysis
                .as_object_mut()
                .ok_or_else(|| ProjectFamilyImportError::invalid_package("章节分析必须是对象"))?;
            let meta = analysis
                .entry("_meta")
                .or_insert_with(|| serde_json::json!({}))
                .as_object_mut()
                .ok_or_else(|| {
                    ProjectFamilyImportError::invalid_package("章节分析 _meta 必须是对象")
                })?;
            meta.insert("project_id".into(), project_id.into());
            meta.insert("chapter_id".into(), id.into());
            let mut needs_reanalysis = false;
            for key in ["planted", "resolved", "progressed"] {
                if let Some(items) = analysis
                    .get_mut("foreshadowing")
                    .and_then(|value| value.get_mut(key))
                    .and_then(Value::as_array_mut)
                {
                    for item in items {
                        if let Some(old_id) = item
                            .get("id")
                            .and_then(Value::as_str)
                            .and_then(|value| value.parse::<i64>().ok())
                        {
                            if let Some(new_id) = maps.foreshadowing_ids.get(&old_id) {
                                item["id"] = Value::String(new_id.to_string());
                            } else {
                                needs_reanalysis = true;
                            }
                        }
                    }
                }
            }
            if needs_reanalysis {
                analysis
                    .get_mut("_meta")
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| {
                        ProjectFamilyImportError::invalid_package("章节分析 _meta 必须是对象")
                    })?
                    .insert("fact_feedback_rebuild_required".into(), Value::Bool(true));
            }
        }
        let mut active: chapter::ActiveModel = current.into();
        active.analysis_data = Set(analysis);
        let current = active.update(tx).await.map_err(db_error)?;
        if let Some(revision) =
            crate::fact_feedback::source_revision(&crate::project::chapter_record(current))
        {
            current_sources.insert(original.id, revision);
        }
    }
    let mut seen = std::collections::HashSet::new();
    for row in &archive.changes {
        if row.project_id != package.root_project_id
            || !seen.insert(row.id)
            || !matches!(row.status.as_str(), "applied" | "proposal" | "retracted")
        {
            return Err(ProjectFamilyImportError::invalid_package(
                "资料事实日志来源、状态或 ID 无效",
            ));
        }
        let mut active: material_fact_change::ActiveModel = row.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(project_id);
        active.workflow_run_id = Set(None);
        active.chapter_id = Set(row
            .chapter_id
            .and_then(|id| maps.chapter_ids.get(&id).copied()));
        let target_map = match row.material_kind.as_str() {
            "world_setting" | "blueprint_synopsis" => Some(&maps.blueprint_ids),
            "character_voice" | "character_arc_progress" => Some(&maps.character_profile_ids),
            "object_profile" => Some(&maps.object_profile_ids),
            "scene_profile" => Some(&maps.scene_profile_ids),
            "foreshadowing" => Some(&maps.foreshadowing_ids),
            "chapter_feedback" => Some(&maps.chapter_ids),
            "planning" => None,
            _ => {
                return Err(ProjectFamilyImportError::invalid_package(
                    "资料事实日志类型无效",
                ));
            }
        };
        active.target_id = Set(match (target_map, row.target_id) {
            (Some(mapping), Some(id)) => match mapping.get(&id) {
                Some(id) => Some(*id),
                None if row.status == "retracted" => None,
                None => {
                    return Err(ProjectFamilyImportError::invalid_package(
                        "事实日志引用不存在的目标",
                    ));
                }
            },
            _ => None,
        });
        let mut sources: Vec<FactSourceRevision> =
            serde_json::from_value(row.source_refs_json.clone())
                .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?;
        for source in &mut sources {
            remap_source(
                source,
                package,
                maps,
                &current_sources,
                row.status == "retracted",
            )?;
        }
        active.source_refs_json = Set(serde_json::to_value(&sources)
            .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?);
        if row.material_kind == "chapter_feedback" {
            if let Some(old_id) = row.chapter_id {
                let original = package
                    .text_core
                    .chapters
                    .iter()
                    .find(|chapter| chapter.id == old_id)
                    .and_then(|chapter| {
                        crate::fact_feedback::source_revision(&crate::project::chapter_record(
                            chapter.clone(),
                        ))
                    });
                let was_current = original.as_ref().is_some_and(|source| {
                    serde_json::to_value(source).ok().as_ref() == Some(&row.after_json)
                });
                if let Some(source) = current_sources.get(&old_id).filter(|_| was_current) {
                    active.after_json = Set(serde_json::to_value(source).map_err(|error| {
                        ProjectFamilyImportError::invalid_package(error.to_string())
                    })?);
                } else {
                    active.status = Set("retracted".into());
                    active.invalidated_at = Set(Some(chrono::Utc::now().naive_utc()));
                }
            }
        }
        let mut evidence = row.evidence_json.clone();
        if let Some(items) = evidence.as_array_mut() {
            for item in items {
                if let Some(id) = item.get("chapter_id").and_then(Value::as_i64) {
                    item["chapter_id"] = maps
                        .chapter_ids
                        .get(&id)
                        .copied()
                        .map(Value::from)
                        .unwrap_or(Value::Null);
                }
            }
        }
        active.evidence_json = Set(evidence);
        active.change_key = Set(format!("import:{project_id}:{}", row.change_key));
        active.insert(tx).await.map_err(db_error)?;
    }
    if let Some(settings) = &archive.settings {
        if settings.project_id != package.root_project_id || settings.interval_chapters <= 0 {
            return Err(ProjectFamilyImportError::invalid_package(
                "事实反馈配置无效",
            ));
        }
        let mut sources: Vec<FactSourceRevision> =
            serde_json::from_value(settings.consolidated_sources_json.clone())
                .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?;
        sources.retain(|source| maps.chapter_ids.contains_key(&source.chapter_id));
        for source in &mut sources {
            remap_source(source, package, maps, &current_sources, false)?;
        }
        project_fact_feedback::ActiveModel {
            project_id: Set(project_id),
            interval_chapters: Set(settings.interval_chapters),
            consolidated_sources_json: Set(serde_json::to_value(sources)
                .map_err(|error| ProjectFamilyImportError::invalid_package(error.to_string()))?),
            last_consolidated_at: Set(settings.last_consolidated_at),
        }
        .insert(tx)
        .await
        .map_err(db_error)?;
    }
    Ok(())
}

fn remap_source(
    source: &mut FactSourceRevision,
    package: &ProjectFamilyImportPackage,
    maps: &ImportIdMaps,
    current: &std::collections::HashMap<i64, FactSourceRevision>,
    historical: bool,
) -> Result<(), ProjectFamilyImportError> {
    let old_id = source.chapter_id;
    if let Some(original) = package
        .text_core
        .chapters
        .iter()
        .find(|chapter| chapter.id == old_id)
    {
        if &crate::fact_feedback::observation_source(&crate::project::chapter_record(
            original.clone(),
        )) == source
        {
            if let Some(revision) = current.get(&old_id) {
                *source = revision.clone();
                return Ok(());
            }
        }
    }
    if let Some(id) = maps.chapter_ids.get(&old_id) {
        source.chapter_id = *id;
    } else if !historical {
        return Err(ProjectFamilyImportError::invalid_package(
            "事实来源章节不存在",
        ));
    }
    Ok(())
}

fn db_error(source: sea_orm::DbErr) -> ProjectFamilyImportError {
    ProjectFamilyImportError::DatabaseContext {
        context: "导入正文事实反馈失败",
        source,
    }
}
