use a4n_db_models::models::{
    character::protagonist_profile,
    project::{material_fact_change, novel_blueprint, novel_project, project_fact_feedback},
};
use a4n_domain::{
    chapter::ChapterRecord,
    fact_feedback::{
        FactChangeOperation, FactMaterialChange, FactMaterialKind, FactSourceRevision,
        apply_fact_json_change, world_setting_document,
    },
    project::BlueprintRecord,
    source_spans::source_spans_for_chapters,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QuerySelect, Set,
    TransactionTrait, sea_query::OnConflict,
};
use serde_json::{Value, json};

use super::{
    FactChangeRecordInput, FactFeedbackDbError, feedback_config, record_fact_change,
    source_revision,
};

pub struct ConsolidationApplyInput {
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub expected_blueprint_version: Option<String>,
    pub sources: Vec<FactSourceRevision>,
    pub changes: Vec<FactMaterialChange>,
}

#[derive(Debug)]
pub struct ConsolidationApplyResult {
    pub changes: Vec<material_fact_change::Model>,
    pub blueprint_version: Option<String>,
}

pub async fn apply_consolidation<T: TransactionTrait>(
    db: &T,
    input: ConsolidationApplyInput,
) -> Result<ConsolidationApplyResult, FactFeedbackDbError> {
    let tx = db.begin().await?;
    let result = apply_in(&tx, input).await?;
    tx.commit().await?;
    Ok(result)
}

async fn apply_in(
    tx: &DatabaseTransaction,
    input: ConsolidationApplyInput,
) -> Result<ConsolidationApplyResult, FactFeedbackDbError> {
    let rows =
        crate::blueprint::locked_blueprint_sources(tx, input.project_id, "归纳正文事实").await?;
    if rows
        .blueprint
        .as_ref()
        .map(|blueprint| &blueprint.current_version)
        != input.expected_blueprint_version.as_ref()
    {
        return Err(FactFeedbackDbError::Invalid(
            "蓝图已变化，请重新归纳正文事实".into(),
        ));
    }
    let chapters = validate_sources(&rows.chapters, input.project_id, &input.sources)?;
    let completed = super::current_feedback_sources(tx, input.project_id).await?;
    if input
        .sources
        .iter()
        .any(|source| !completed.iter().any(|current| &current.revision == source))
    {
        return Err(FactFeedbackDbError::Invalid(
            "章节事实反馈已失效，请重新同步后归纳".into(),
        ));
    }
    let spans = source_spans_for_chapters(&chapters);
    let mut blueprint = rows.blueprint.clone();
    let mut blueprint_changed = false;
    let mut recorded = Vec::new();
    for mut change in input.changes {
        change.validate().map_err(FactFeedbackDbError::Invalid)?;
        let evidence = change
            .source_span_ids
            .iter()
            .map(|id| {
                spans.iter().find(|span| &span.id == id).ok_or_else(|| {
                    FactFeedbackDbError::Invalid(format!("资料变更引用未知正文证据: {id}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sources = input
            .sources
            .iter()
            .filter(|source| {
                evidence
                    .iter()
                    .any(|span| span.chapter_id == source.chapter_id)
            })
            .cloned()
            .collect();
        let mut proposal = change.material_kind == FactMaterialKind::Planning;
        if change.material_kind == FactMaterialKind::WorldSetting {
            let allowed: std::collections::BTreeSet<_> = chapters
                .iter()
                .flat_map(|chapter| {
                    chapter
                        .analysis_data
                        .as_ref()
                        .and_then(|analysis| analysis.get("world_facts"))
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter(|fact| {
                    let field = change
                        .path
                        .strip_prefix('/')
                        .and_then(|path| path.split('/').next());
                    fact.get("field").and_then(Value::as_str) == field
                        && (fact.get("assertion").and_then(Value::as_str) == Some("observed")
                            || (change.path.starts_with("/history_timeline")
                                && fact.get("assertion").and_then(Value::as_str)
                                    == Some("recollection")))
                })
                .flat_map(|fact| {
                    fact.get("source_span_ids")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                })
                .filter_map(Value::as_str)
                .collect();
            if change
                .source_span_ids
                .iter()
                .any(|id| !allowed.contains(id.as_str()))
            {
                return Err(FactFeedbackDbError::Invalid(
                    "世界观变更只能引用已识别的明确世界事实；传闻和假设不能写回".into(),
                ));
            }
        }
        let changed = match change.material_kind {
            FactMaterialKind::WorldSetting => {
                if let Some(current) = blueprint.as_mut() {
                    if change.target_id != Some(current.id) {
                        return Err(FactFeedbackDbError::Invalid(
                            "资料变更蓝图 ID 与当前项目不一致".into(),
                        ));
                    }
                    let mut world = world_setting_document(current.world_setting.as_ref());
                    let appended_path = if change.operation == FactChangeOperation::Add
                        && change.path.ends_with("/-")
                    {
                        let parent = change.path.strip_suffix("/-").unwrap();
                        world
                            .pointer(parent)
                            .and_then(Value::as_array)
                            .map(|items| format!("{parent}/{}", items.len()))
                    } else {
                        None
                    };
                    let removed_array = if change.operation == FactChangeOperation::Remove {
                        change.path.rsplit_once('/').and_then(|(parent, _)| {
                            world
                                .pointer(parent)
                                .filter(|value| value.is_array())
                                .cloned()
                                .map(|value| (parent.to_string(), value))
                        })
                    } else {
                        None
                    };
                    let changed = apply_fact_json_change(&mut world, &change)
                        .map_err(FactFeedbackDbError::Invalid)?;
                    if let Some(path) = appended_path {
                        change.path = path;
                    }
                    if let Some((parent, before)) = removed_array {
                        change.after = world.pointer(&parent).cloned().ok_or_else(|| {
                            FactFeedbackDbError::Invalid("移除后数组父节点不存在".into())
                        })?;
                        change.path = parent;
                        change.before = before;
                        change.operation = FactChangeOperation::Replace;
                    }
                    if changed {
                        current.world_setting = Some(world);
                    }
                    blueprint_changed |= changed;
                    changed
                } else {
                    if change.target_id.is_some() {
                        return Err(FactFeedbackDbError::Invalid(
                            "项目没有蓝图，变更只能保留为草案".into(),
                        ));
                    }
                    proposal = true;
                    true
                }
            }
            FactMaterialKind::BlueprintSynopsis => {
                // A free-form synopsis mixes established events and future intent; keep its rewrite reviewable.
                proposal = true;
                if !change.after.is_string() && !change.after.is_null() {
                    return Err(FactFeedbackDbError::Invalid("蓝图梗概必须是文本".into()));
                }
                if let Some(current) = blueprint.as_ref() {
                    if change.target_id != Some(current.id) {
                        return Err(FactFeedbackDbError::Invalid(
                            "剧情梗概建议的蓝图 ID 与当前项目不一致".into(),
                        ));
                    }
                    let mut synopsis = current
                        .full_synopsis
                        .as_ref()
                        .map(|text| json!(text))
                        .unwrap_or(Value::Null);
                    apply_fact_json_change(&mut synopsis, &change)
                        .map_err(FactFeedbackDbError::Invalid)?
                } else if change.target_id.is_some() {
                    return Err(FactFeedbackDbError::Invalid(
                        "项目没有蓝图，梗概建议不能引用蓝图 ID".into(),
                    ));
                } else {
                    true
                }
            }
            FactMaterialKind::CharacterVoice | FactMaterialKind::CharacterArcProgress => {
                apply_character_change(tx, input.project_id, &change).await?
            }
            FactMaterialKind::Planning => true,
        };
        if !changed {
            continue;
        }
        recorded.push(record_fact_change(tx, FactChangeRecordInput {
            project_id: input.project_id, workflow_run_id: Some(input.workflow_run_id),
            material_kind: change.material_kind.as_str().into(), target_id: change.target_id,
            field_path: change.path, operation: match change.operation { FactChangeOperation::Add => "add", FactChangeOperation::Replace => "replace", FactChangeOperation::Remove => "remove" }.into(),
            before: change.before, after: change.after, sources,
            evidence: Value::Array(evidence.into_iter().map(|span| json!({"id": span.id, "chapter_id": span.chapter_id, "chapter_number": span.chapter_number, "quote": span.text})).collect()),
            reason: change.reason, status: if proposal { "proposal" } else { "applied" }.into(),
        }).await?);
    }
    let blueprint_version = if blueprint_changed {
        let updated = persist_blueprint(tx, blueprint.unwrap(), &rows.project).await?;
        Some(updated.current_version)
    } else {
        None
    };
    let config = feedback_config(tx, input.project_id).await?;
    project_fact_feedback::Entity::insert(project_fact_feedback::ActiveModel {
        project_id: Set(input.project_id),
        interval_chapters: Set(config.interval_chapters),
        consolidated_sources_json: Set(serde_json::to_value(&input.sources)?),
        last_consolidated_at: Set(Some(Utc::now().naive_utc())),
    })
    .on_conflict(
        OnConflict::column(project_fact_feedback::Column::ProjectId)
            .update_columns([
                project_fact_feedback::Column::ConsolidatedSourcesJson,
                project_fact_feedback::Column::LastConsolidatedAt,
            ])
            .to_owned(),
    )
    .exec(tx)
    .await?;
    crate::workflow::upsert_workflow_checkpoint_in_transaction(tx, crate::workflow::UpsertWorkflowCheckpointInput {
        workflow_run_id: input.workflow_run_id, step_key: a4n_domain::fact_feedback::FACT_APPLY_STEP.into(), step_order: 2,
        status: "completed".into(), input_fingerprint_sha256: None,
        input_fingerprint_json: Some(json!({"sources":input.sources})),
        checkpoint_json: Some(json!({"completed":true,"blueprint_version":blueprint_version,
            "characters_changed":recorded.iter().any(|change| change.status == "applied" && matches!(change.material_kind.as_str(), "character_voice" | "character_arc_progress"))})),
        artifacts_json: Some(json!({"change_ids":recorded.iter().map(|change|change.id).collect::<Vec<_>>()})),
        generation_run_ids_json: None, error_message: None, completed: true,
    }).await?;
    Ok(ConsolidationApplyResult {
        changes: recorded,
        blueprint_version,
    })
}

fn validate_sources(
    chapters: &[ChapterRecord],
    project_id: i64,
    sources: &[FactSourceRevision],
) -> Result<Vec<ChapterRecord>, FactFeedbackDbError> {
    if sources.is_empty() {
        return Err(FactFeedbackDbError::Invalid(
            "没有可归纳的已分析正文".into(),
        ));
    }
    sources
        .iter()
        .map(|source| {
            chapters
                .iter()
                .find(|chapter| {
                    chapter.id == source.chapter_id
                        && chapter.project_id == project_id
                        && source_revision(chapter).as_ref() == Some(source)
                })
                .cloned()
                .ok_or_else(|| {
                    FactFeedbackDbError::Invalid(format!(
                        "第 {} 章正文或分析已变化，请重新归纳",
                        source.chapter_number
                    ))
                })
        })
        .collect()
}

async fn apply_character_change(
    tx: &DatabaseTransaction,
    project_id: i64,
    change: &FactMaterialChange,
) -> Result<bool, FactFeedbackDbError> {
    let id = change
        .target_id
        .ok_or_else(|| FactFeedbackDbError::Invalid("角色归纳缺少 target_id".into()))?;
    let profile = protagonist_profile::Entity::find_by_id(id)
        .lock_exclusive()
        .one(tx)
        .await?
        .filter(|profile| profile.project_id == project_id)
        .ok_or_else(|| FactFeedbackDbError::Invalid("角色不属于当前项目".into()))?;
    let kind = change.material_kind.as_str();
    let mut value = character_enrichment(&profile, kind)?;
    if !change.after.is_object() && !change.after.is_null() {
        return Err(FactFeedbackDbError::Invalid(
            "角色说话特征和弧线进展必须是对象".into(),
        ));
    }
    let changed =
        apply_fact_json_change(&mut value, change).map_err(FactFeedbackDbError::Invalid)?;
    if !changed {
        return Ok(false);
    }
    persist_character_enrichment(tx, profile, kind, value).await?;
    Ok(true)
}

pub(super) fn character_enrichment(
    profile: &protagonist_profile::Model,
    kind: &str,
) -> Result<Value, FactFeedbackDbError> {
    match kind {
        "character_voice" => Ok(profile.voice_fingerprint.clone().unwrap_or(Value::Null)),
        "character_arc_progress" => match profile.character_arc.as_ref() {
            None => Ok(Value::Null),
            Some(Value::Object(arc)) => {
                Ok(arc.get("observed_progress").cloned().unwrap_or(Value::Null))
            }
            _ => Err(FactFeedbackDbError::Invalid("角色弧线结构无效".into())),
        },
        _ => Err(FactFeedbackDbError::Invalid(format!(
            "未知角色资料类型: {kind}"
        ))),
    }
}

pub(super) fn set_character_enrichment(
    profile: &mut protagonist_profile::Model,
    kind: &str,
    value: Value,
) -> Result<(), FactFeedbackDbError> {
    match kind {
        "character_voice" => profile.voice_fingerprint = (!value.is_null()).then_some(value),
        "character_arc_progress" => {
            if value.is_null() && profile.character_arc.is_none() {
                return Ok(());
            }
            let object = profile
                .character_arc
                .get_or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or_else(|| FactFeedbackDbError::Invalid("角色弧线结构无效".into()))?;
            if value.is_null() {
                object.remove("observed_progress");
            } else {
                object.insert("observed_progress".into(), value);
            }
        }
        _ => {
            return Err(FactFeedbackDbError::Invalid(format!(
                "未知角色资料类型: {kind}"
            )));
        }
    }
    Ok(())
}

pub(super) async fn persist_character_enrichment<C: ConnectionTrait>(
    tx: &C,
    mut profile: protagonist_profile::Model,
    kind: &str,
    value: Value,
) -> Result<(), FactFeedbackDbError> {
    set_character_enrichment(&mut profile, kind, value)?;
    protagonist_profile::ActiveModel {
        id: Set(profile.id),
        voice_fingerprint: Set(profile.voice_fingerprint),
        character_arc: Set(profile.character_arc),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(tx)
    .await?;
    Ok(())
}

pub(super) async fn persist_blueprint<C: ConnectionTrait>(
    tx: &C,
    mut blueprint: BlueprintRecord,
    project: &a4n_domain::project::ProjectRecord,
) -> Result<BlueprintRecord, FactFeedbackDbError> {
    let now = Utc::now().naive_utc();
    blueprint.current_version = crate::blueprint::next_blueprint_version_record(
        tx,
        blueprint.project_id,
        Some(&blueprint.current_version),
    )
    .await?;
    blueprint.updated_at = now;
    let mut active: novel_blueprint::ActiveModel =
        crate::project::blueprint_model_from_record(blueprint.clone()).into();
    active.world_setting = Set(blueprint.world_setting.clone());
    active.full_synopsis = Set(blueprint.full_synopsis.clone());
    active.current_version = Set(blueprint.current_version.clone());
    active.updated_at = Set(now);
    let blueprint = crate::project::blueprint_record(active.update(tx).await?);
    let characters = crate::project::blueprint_characters_for_project(tx, project.id).await?;
    let relationships = crate::project::blueprint_relationships_for_project(tx, project.id).await?;
    let snapshot =
        crate::blueprint::blueprint_version_snapshot(&blueprint, &characters, &relationships);
    crate::blueprint::insert_blueprint_version_record(
        tx,
        crate::blueprint::InsertBlueprintVersionInput {
            project_id: project.id,
            version: blueprint.current_version.clone(),
            change_description: "正文事实增量反馈".into(),
            blueprint_data: snapshot.clone(),
            now,
            created_by: "body_fact_feedback".into(),
        },
    )
    .await?;
    // Feedback refreshes the blueprint cache, without resetting the author's project settings.
    novel_project::ActiveModel {
        id: Set(project.id),
        raw_blueprint: Set(Some(serde_json::to_string_pretty(&snapshot)?)),
        updated_at: Set(now),
        ..Default::default()
    }
    .update(tx)
    .await?;
    Ok(blueprint)
}
