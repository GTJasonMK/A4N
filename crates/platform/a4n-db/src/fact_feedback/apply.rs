use a4n_db_models::models::{
    chapter::chapter,
    character::protagonist_profile,
    project::{material_fact_change, novel_blueprint, project_fact_feedback},
};
use a4n_domain::{
    fact_feedback::{
        FactChangeOperation, FactMaterialChange, FactMaterialKind, FactSourceRevision,
        apply_fact_json_change,
    },
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
                    fact.get("assertion").and_then(Value::as_str) == Some("observed")
                        || (change.path.starts_with("/history_timeline")
                            && fact.get("assertion").and_then(Value::as_str)
                                == Some("recollection"))
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
            FactMaterialKind::WorldSetting | FactMaterialKind::BlueprintSynopsis => {
                if let Some(current) = blueprint.as_mut() {
                    if change.target_id != Some(current.id) {
                        return Err(FactFeedbackDbError::Invalid(
                            "资料变更蓝图 ID 与当前项目不一致".into(),
                        ));
                    }
                    let appended_path = if change.operation == FactChangeOperation::Add
                        && change.path.ends_with("/-")
                    {
                        let parent = change.path.strip_suffix("/-").unwrap();
                        world_document(current.world_setting.as_ref())
                            .pointer(parent)
                            .and_then(Value::as_array)
                            .map(|items| format!("{parent}/{}", items.len()))
                    } else {
                        None
                    };
                    let removed_array = if change.operation == FactChangeOperation::Remove {
                        change.path.rsplit_once('/').and_then(|(parent, _)| {
                            world_document(current.world_setting.as_ref())
                                .pointer(parent)
                                .filter(|value| value.is_array())
                                .cloned()
                                .map(|value| (parent.to_string(), value))
                        })
                    } else {
                        None
                    };
                    let changed = apply_blueprint_change(current, &change)?;
                    if let Some(path) = appended_path {
                        change.path = path;
                    }
                    if let Some((parent, before)) = removed_array {
                        change.after = world_document(current.world_setting.as_ref())
                            .pointer(&parent)
                            .cloned()
                            .ok_or_else(|| {
                                FactFeedbackDbError::Invalid("移除后数组父节点不存在".into())
                            })?;
                        change.path = parent;
                        change.before = before;
                        change.operation = FactChangeOperation::Replace;
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
    chapters: &[chapter::Model],
    project_id: i64,
    sources: &[FactSourceRevision],
) -> Result<Vec<chapter::Model>, FactFeedbackDbError> {
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

pub(super) fn world_document(value: Option<&Value>) -> Value {
    match value {
        Some(Value::String(text)) => json!({"premise": text}),
        Some(value) if !value.is_null() => value.clone(),
        _ => json!({}),
    }
}

fn apply_blueprint_change(
    blueprint: &mut novel_blueprint::Model,
    change: &FactMaterialChange,
) -> Result<bool, FactFeedbackDbError> {
    if change.material_kind == FactMaterialKind::WorldSetting {
        let mut world = world_document(blueprint.world_setting.as_ref());
        let changed =
            apply_fact_json_change(&mut world, change).map_err(FactFeedbackDbError::Invalid)?;
        if changed {
            blueprint.world_setting = Some(world);
        }
        return Ok(changed);
    }
    let mut synopsis = blueprint
        .full_synopsis
        .as_ref()
        .map(|text| json!(text))
        .unwrap_or(Value::Null);
    if !change.after.is_string() && !change.after.is_null() {
        return Err(FactFeedbackDbError::Invalid("蓝图梗概必须是文本".into()));
    }
    let changed =
        apply_fact_json_change(&mut synopsis, change).map_err(FactFeedbackDbError::Invalid)?;
    if changed {
        blueprint.full_synopsis = synopsis.as_str().map(str::to_string);
    }
    Ok(changed)
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
    let mut value = if change.material_kind == FactMaterialKind::CharacterVoice {
        profile.voice_fingerprint.clone().unwrap_or(Value::Null)
    } else {
        profile
            .character_arc
            .as_ref()
            .and_then(|arc| arc.get("observed_progress"))
            .cloned()
            .unwrap_or(Value::Null)
    };
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
    let mut active: protagonist_profile::ActiveModel = profile.clone().into();
    if change.material_kind == FactMaterialKind::CharacterVoice {
        active.voice_fingerprint = Set((!value.is_null()).then_some(value));
    } else {
        let mut arc = profile.character_arc.unwrap_or_else(|| json!({}));
        let object = arc.as_object_mut().ok_or_else(|| {
            FactFeedbackDbError::Invalid("已有角色弧线不是结构化对象，请先核对资料".into())
        })?;
        if value.is_null() {
            object.remove("observed_progress");
        } else {
            object.insert("observed_progress".into(), value);
        }
        active.character_arc = Set(Some(arc));
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(tx).await?;
    Ok(true)
}

pub(super) async fn persist_blueprint<C: ConnectionTrait>(
    tx: &C,
    mut blueprint: novel_blueprint::Model,
    project: &a4n_db_models::models::project::novel_project::Model,
) -> Result<novel_blueprint::Model, FactFeedbackDbError> {
    let now = Utc::now().naive_utc();
    blueprint.current_version = crate::blueprint::next_blueprint_version_record(
        tx,
        blueprint.project_id,
        Some(&blueprint.current_version),
    )
    .await?;
    blueprint.updated_at = now;
    let mut active: novel_blueprint::ActiveModel = blueprint.clone().into();
    active.world_setting = Set(blueprint.world_setting.clone());
    active.full_synopsis = Set(blueprint.full_synopsis.clone());
    active.current_version = Set(blueprint.current_version.clone());
    active.updated_at = Set(now);
    let blueprint = active.update(tx).await?;
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
    let status = a4n_domain::project::ProjectStatus::parse(&project.status)
        .ok_or_else(|| FactFeedbackDbError::Invalid("项目状态无效".into()))?;
    let total = blueprint
        .total_chapters
        .ok_or_else(|| FactFeedbackDbError::Invalid("蓝图缺少章节计划".into()))?;
    crate::blueprint::sync_project_after_blueprint_record(
        tx,
        crate::blueprint::SyncProjectAfterBlueprintInput {
            project_id: project.id,
            title: blueprint.title.clone(),
            status,
            target_chapters: total,
            total_chapters: Some(total),
            chapters_per_part: blueprint.chapters_per_part,
            raw_blueprint: serde_json::to_string_pretty(&snapshot)?,
            now,
        },
    )
    .await?;
    Ok(blueprint)
}
