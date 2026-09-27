//! Unified project-bundle history transfer.
//!
//! The regular family/canvas importers own their domain rows. This module owns
//! the cross-cutting conversation and generation history rows that must follow
//! those projects without carrying runtime ownership to another machine.

use std::collections::{HashMap, HashSet};

use a4n_db_models::models::{
    generation::{
        generation_run, generation_run_target, llm_call_trace, model_usage_record,
        workflow_checkpoint, workflow_progress_event, workflow_run,
    },
    user::{conversation, message},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum ProjectBundleHistoryError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sea_orm::DbErr),
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectBundleHistoryRows {
    pub conversations: Vec<conversation::Model>,
    pub messages: Vec<message::Model>,
    pub workflow_runs: Vec<workflow_run::Model>,
    pub workflow_checkpoints: Vec<workflow_checkpoint::Model>,
    pub workflow_progress_events: Vec<workflow_progress_event::Model>,
    pub generation_runs: Vec<generation_run::Model>,
    pub generation_run_targets: Vec<generation_run_target::Model>,
    pub llm_call_traces: Vec<llm_call_trace::Model>,
    pub model_usage_records: Vec<model_usage_record::Model>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectBundleEntityIdMaps {
    pub project_ids: HashMap<i64, i64>,
    pub root_project_ids: HashMap<i64, i64>,
    pub chapter_ids: HashMap<i64, i64>,
    pub source_snapshot_ids: HashMap<i64, i64>,
    pub source_snapshot_chapter_ids: HashMap<i64, i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectBundleHistoryImportSummary {
    pub conversations_imported: usize,
    pub messages_imported: usize,
    pub workflow_runs_imported: usize,
    pub workflow_runs_marked_interrupted: usize,
    pub generation_runs_imported: usize,
}

fn selected_project_scope<C: ColumnTrait>(project_id: C, project_ids: &[i64]) -> Condition {
    // Root lineage must not pull an excluded derived project's history into the bundle.
    Condition::any()
        .add(project_id.is_null())
        .add(project_id.is_in(project_ids.iter().copied()))
}

pub async fn export_project_bundle_history<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<ProjectBundleHistoryRows, ProjectBundleHistoryError>
where
    C: ConnectionTrait,
{
    let project_ids = unique_positive_ids(project_ids, "project_id")?;
    if project_ids.is_empty() {
        return Ok(ProjectBundleHistoryRows::default());
    }

    let conversations = conversation::Entity::find()
        .filter(conversation::Column::ProjectId.is_in(project_ids.clone()))
        .order_by_asc(conversation::Column::Id)
        .all(db)
        .await?;
    let conversation_ids = conversations.iter().map(|row| row.id).collect::<Vec<_>>();
    let messages = if conversation_ids.is_empty() {
        Vec::new()
    } else {
        message::Entity::find()
            .filter(message::Column::ConversationId.is_in(conversation_ids))
            .order_by_asc(message::Column::ConversationId)
            .order_by_asc(message::Column::CreatedAt)
            .order_by_asc(message::Column::Id)
            .all(db)
            .await?
    };

    let project_condition = Condition::any()
        .add(workflow_run::Column::ProjectId.is_in(project_ids.clone()))
        .add(workflow_run::Column::RootProjectId.is_in(project_ids.clone()));
    let workflow_runs = workflow_run::Entity::find()
        .filter(project_condition)
        .filter(selected_project_scope(
            workflow_run::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(workflow_run::Column::Id)
        .all(db)
        .await?;
    let workflow_ids = workflow_runs.iter().map(|row| row.id).collect::<Vec<_>>();
    let workflow_checkpoints = if workflow_ids.is_empty() {
        Vec::new()
    } else {
        workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.is_in(workflow_ids.clone()))
            .order_by_asc(workflow_checkpoint::Column::WorkflowRunId)
            .order_by_asc(workflow_checkpoint::Column::StepOrder)
            .order_by_asc(workflow_checkpoint::Column::Id)
            .all(db)
            .await?
    };
    let workflow_progress_events = if workflow_ids.is_empty() {
        Vec::new()
    } else {
        workflow_progress_event::Entity::find()
            .filter(workflow_progress_event::Column::WorkflowRunId.is_in(workflow_ids.clone()))
            .order_by_asc(workflow_progress_event::Column::WorkflowRunId)
            .order_by_asc(workflow_progress_event::Column::EventIndex)
            .order_by_asc(workflow_progress_event::Column::Id)
            .all(db)
            .await?
    };

    let generation_condition = Condition::any()
        .add(generation_run::Column::ProjectId.is_in(project_ids.clone()))
        .add(generation_run::Column::RootProjectId.is_in(project_ids.clone()));
    let generation_condition = if workflow_ids.is_empty() {
        generation_condition
    } else {
        generation_condition.add(generation_run::Column::WorkflowRunId.is_in(workflow_ids.clone()))
    };
    let generation_runs = generation_run::Entity::find()
        .filter(generation_condition)
        .filter(selected_project_scope(
            generation_run::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(generation_run::Column::Id)
        .all(db)
        .await?;
    let generation_ids = generation_runs.iter().map(|row| row.id).collect::<Vec<_>>();
    let generation_run_targets = if generation_ids.is_empty() {
        Vec::new()
    } else {
        generation_run_target::Entity::find()
            .filter(generation_run_target::Column::GenerationRunId.is_in(generation_ids.clone()))
            .order_by_asc(generation_run_target::Column::GenerationRunId)
            .order_by_asc(generation_run_target::Column::Id)
            .all(db)
            .await?
    };

    let llm_trace_condition = Condition::any()
        .add(llm_call_trace::Column::ProjectId.is_in(project_ids.clone()))
        .add(llm_call_trace::Column::RootProjectId.is_in(project_ids.clone()));
    let llm_trace_condition = if workflow_ids.is_empty() {
        llm_trace_condition
    } else {
        llm_trace_condition.add(llm_call_trace::Column::WorkflowRunId.is_in(workflow_ids.clone()))
    };
    let llm_trace_condition = if generation_ids.is_empty() {
        llm_trace_condition
    } else {
        llm_trace_condition
            .add(llm_call_trace::Column::GenerationRunId.is_in(generation_ids.clone()))
    };
    let llm_call_traces = llm_call_trace::Entity::find()
        .filter(llm_trace_condition)
        .filter(selected_project_scope(
            llm_call_trace::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(llm_call_trace::Column::Id)
        .all(db)
        .await?;

    let usage_condition = Condition::any()
        .add(model_usage_record::Column::ProjectId.is_in(project_ids.clone()))
        .add(model_usage_record::Column::RootProjectId.is_in(project_ids.clone()));
    let usage_condition = if workflow_ids.is_empty() {
        usage_condition
    } else {
        usage_condition.add(model_usage_record::Column::WorkflowRunId.is_in(workflow_ids.clone()))
    };
    let usage_condition = if generation_ids.is_empty() {
        usage_condition
    } else {
        usage_condition.add(model_usage_record::Column::GenerationRunId.is_in(generation_ids))
    };
    let model_usage_records = model_usage_record::Entity::find()
        .filter(usage_condition)
        .filter(selected_project_scope(
            model_usage_record::Column::ProjectId,
            &project_ids,
        ))
        .order_by_asc(model_usage_record::Column::Id)
        .all(db)
        .await?;

    Ok(ProjectBundleHistoryRows {
        conversations,
        messages,
        workflow_runs,
        workflow_checkpoints,
        workflow_progress_events,
        generation_runs,
        generation_run_targets,
        llm_call_traces,
        model_usage_records,
    })
}

pub async fn import_project_bundle_history(
    db: &DatabaseConnection,
    rows: &ProjectBundleHistoryRows,
    maps: &ProjectBundleEntityIdMaps,
    bundle_id: &str,
    user_id: i64,
) -> Result<ProjectBundleHistoryImportSummary, ProjectBundleHistoryError> {
    let bundle_id = format!(
        "{}:{}",
        required_bundle_id(bundle_id)?,
        uuid::Uuid::new_v4()
    );
    if user_id <= 0 {
        return Err(ProjectBundleHistoryError::Invalid(
            "user_id 必须大于 0".to_string(),
        ));
    }
    let txn = db.begin().await?;
    let mut summary = ProjectBundleHistoryImportSummary::default();
    let mut conversation_ids = HashMap::new();
    for source in &rows.conversations {
        let project_id = map_required(
            &maps.project_ids,
            source.project_id,
            "conversation.project_id",
        )?;
        let mut active: conversation::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(project_id);
        let inserted = active.insert(&txn).await?;
        conversation_ids.insert(source.id, inserted.id);
        summary.conversations_imported += 1;
    }
    for source in &rows.messages {
        let conversation_id = map_required(
            &conversation_ids,
            source.conversation_id,
            "message.conversation_id",
        )?;
        let mut active: message::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.conversation_id = Set(conversation_id);
        active.insert(&txn).await?;
        summary.messages_imported += 1;
    }

    let mut workflow_ids = HashMap::new();
    for source in &rows.workflow_runs {
        let project_id = source
            .project_id
            .map(|id| map_required(&maps.project_ids, id, "workflow_run.project_id"))
            .transpose()?;
        let root_project_id = source
            .root_project_id
            .map(|id| map_required(&maps.root_project_ids, id, "workflow_run.root_project_id"))
            .transpose()?;
        let chapter_id = source
            .chapter_id
            .map(|id| map_required(&maps.chapter_ids, id, "workflow_run.chapter_id"))
            .transpose()?;
        let source_snapshot_id = source
            .source_snapshot_id
            .map(|id| {
                map_required(
                    &maps.source_snapshot_ids,
                    id,
                    "workflow_run.source_snapshot_id",
                )
            })
            .transpose()?;
        let (status, can_resume, resume_block_reason, interrupted) = imported_status(
            &source.status,
            source.can_resume,
            source.resume_block_reason.clone(),
        );
        let (scope_table, scope_id, scope_is_mapped) =
            remap_workflow_scope(source.scope_table.as_deref(), source.scope_id, maps);
        let (can_resume, resume_block_reason) = if scope_is_mapped || !can_resume {
            (can_resume, resume_block_reason)
        } else {
            (
                false,
                Some("原机器工作流作用域无法在目标项目中映射；请重新提交当前步骤".to_string()),
            )
        };
        let mut active: workflow_run::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.business_key = Set(format!("{bundle_id}:{}", source.business_key));
        active.project_id = Set(project_id);
        active.root_project_id = Set(root_project_id);
        active.source_snapshot_id = Set(source_snapshot_id);
        active.scope_table = Set(scope_table);
        active.scope_id = Set(scope_id);
        active.chapter_id = Set(chapter_id);
        active.input_fingerprint_json = Set(remap_history_json(
            source.input_fingerprint_json.clone(),
            maps,
        )?);
        active.status = Set(status);
        active.active_task_id = Set(None);
        active.can_resume = Set(can_resume);
        active.resume_block_reason = Set(resume_block_reason);
        active.completed_at = Set(if can_resume {
            None
        } else {
            source.completed_at
        });
        let inserted = active.insert(&txn).await?;
        workflow_ids.insert(source.id, inserted.id);
        summary.workflow_runs_imported += 1;
        if interrupted {
            summary.workflow_runs_marked_interrupted += 1;
        }
    }

    let mut generation_ids = HashMap::new();
    for source in &rows.generation_runs {
        let project_id = map_required(
            &maps.project_ids,
            source.project_id,
            "generation_run.project_id",
        )?;
        let root_project_id = map_required(
            &maps.root_project_ids,
            source.root_project_id,
            "generation_run.root_project_id",
        )?;
        let workflow_run_id = source
            .workflow_run_id
            .map(|id| map_required(&workflow_ids, id, "generation_run.workflow_run_id"))
            .transpose()?;
        let (status, _, _, _) = imported_status(&source.status, false, None);
        let mut active: generation_run::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(project_id);
        active.root_project_id = Set(root_project_id);
        active.workflow_run_id = Set(workflow_run_id);
        active.status = Set(status);
        active.request_json = Set(sanitize_runtime_json(source.request_json.clone()));
        active.raw_response_json = Set(sanitize_runtime_json(source.raw_response_json.clone()));
        active.completed_at = Set(if source.status == "running" {
            None
        } else {
            source.completed_at
        });
        let inserted = active.insert(&txn).await?;
        generation_ids.insert(source.id, inserted.id);
        summary.generation_runs_imported += 1;
    }

    for source in &rows.generation_run_targets {
        let generation_run_id = map_required(
            &generation_ids,
            source.generation_run_id,
            "generation_run_target.generation_run_id",
        )?;
        let mut active: generation_run_target::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.generation_run_id = Set(generation_run_id);
        active.insert(&txn).await?;
    }

    for source in &rows.workflow_checkpoints {
        let workflow_run_id = map_required(
            &workflow_ids,
            source.workflow_run_id,
            "workflow_checkpoint.workflow_run_id",
        )?;
        let mut active: workflow_checkpoint::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.workflow_run_id = Set(workflow_run_id);
        if let Some(artifacts) = source.artifacts_json.as_ref().filter(|artifacts| {
            artifacts
                .get(a4n_domain::shooting_script::compat::checkpoint::TIMING_UPGRADE_KEY)
                .is_some()
        }) {
            let original = source.checkpoint_json.as_ref().ok_or_else(|| {
                ProjectBundleHistoryError::Invalid(format!(
                    "工作流断点 {} 的升级回执缺少输出",
                    source.id
                ))
            })?;
            a4n_domain::shooting_script::compat::checkpoint::validate_checkpoint_upgrade(
                original, artifacts,
            )
            .map_err(ProjectBundleHistoryError::Invalid)?;
        }
        let (input_fingerprint, fingerprint_hash) = remap_checkpoint_input(
            source.input_fingerprint_json.clone(),
            source.input_fingerprint_sha256.as_deref(),
            maps,
        )
        .map_err(|error| {
            ProjectBundleHistoryError::Invalid(format!("工作流断点 {}: {error}", source.id))
        })?;
        active.input_fingerprint_sha256 = Set(fingerprint_hash);
        active.input_fingerprint_json = Set(input_fingerprint);
        let checkpoint = remap_history_json(source.checkpoint_json.clone(), maps)?;
        let mut artifacts = remap_history_json(source.artifacts_json.clone(), maps)?;
        if let Some(receipt) = artifacts.as_mut().and_then(|value| {
            value.get_mut(a4n_domain::shooting_script::compat::checkpoint::TIMING_UPGRADE_KEY)
        }) {
            receipt["upgraded_input_fingerprint_sha256"] = json!(
                crate::generation::generation_checkpoint_fingerprint_sha256(
                    &receipt["upgraded_input_fingerprint_json"]
                )
                .map_err(|e| ProjectBundleHistoryError::Invalid(e.to_string()))?
            );
            receipt["upgraded_parsed_output_sha256"] = json!(
                crate::generation::generation_checkpoint_fingerprint_sha256(
                    &checkpoint
                        .as_ref()
                        .ok_or_else(|| ProjectBundleHistoryError::Invalid(
                            "升级断点缺少结构化输出".into()
                        ))?["parsed_output"]
                )
                .map_err(|e| ProjectBundleHistoryError::Invalid(e.to_string()))?
            );
        }
        active.checkpoint_json = Set(checkpoint);
        active.artifacts_json = Set(artifacts);
        active.generation_run_ids_json = Set(remap_generation_ids(
            source.generation_run_ids_json.clone(),
            &generation_ids,
        )?);
        active.status = Set(if source.status == "running" {
            "interrupted".to_string()
        } else {
            source.status.clone()
        });
        active.completed_at = Set(if source.status == "running" {
            None
        } else {
            source.completed_at
        });
        active.insert(&txn).await?;
    }
    for source in &rows.workflow_progress_events {
        let workflow_run_id = map_required(
            &workflow_ids,
            source.workflow_run_id,
            "workflow_progress_event.workflow_run_id",
        )?;
        let mut active: workflow_progress_event::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.workflow_run_id = Set(workflow_run_id);
        active.task_id = Set(None);
        active.meta_json = Set(remap_history_json(source.meta_json.clone(), maps)?);
        active.insert(&txn).await?;
    }
    for source in &rows.llm_call_traces {
        let mut active: llm_call_trace::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(source
            .project_id
            .map(|id| map_required(&maps.project_ids, id, "llm_call_trace.project_id"))
            .transpose()?);
        active.root_project_id = Set(source
            .root_project_id
            .map(|id| map_required(&maps.root_project_ids, id, "llm_call_trace.root_project_id"))
            .transpose()?);
        active.workflow_run_id = Set(source
            .workflow_run_id
            .map(|id| map_required(&workflow_ids, id, "llm_call_trace.workflow_run_id"))
            .transpose()?);
        active.generation_run_id = Set(source
            .generation_run_id
            .map(|id| map_required(&generation_ids, id, "llm_call_trace.generation_run_id"))
            .transpose()?);
        active.context_plan_json = Set(sanitize_runtime_json(source.context_plan_json.clone()));
        active.context_pack_refs_json =
            Set(sanitize_runtime_json(source.context_pack_refs_json.clone()));
        active.insert(&txn).await?;
    }
    for source in &rows.model_usage_records {
        let mut active: model_usage_record::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.request_id = Set(uuid::Uuid::new_v4().to_string());
        active.user_id = Set(Some(user_id));
        active.project_id = Set(source
            .project_id
            .map(|id| map_required(&maps.project_ids, id, "model_usage_record.project_id"))
            .transpose()?);
        active.root_project_id = Set(source
            .root_project_id
            .map(|id| {
                map_required(
                    &maps.root_project_ids,
                    id,
                    "model_usage_record.root_project_id",
                )
            })
            .transpose()?);
        active.workflow_run_id = Set(source
            .workflow_run_id
            .map(|id| map_required(&workflow_ids, id, "model_usage_record.workflow_run_id"))
            .transpose()?);
        active.generation_run_id = Set(source
            .generation_run_id
            .map(|id| map_required(&generation_ids, id, "model_usage_record.generation_run_id"))
            .transpose()?);
        active.provider_config_id = Set(None);
        active.pricing_snapshot_json = Set(sanitize_runtime_json(Some(
            source.pricing_snapshot_json.clone(),
        ))
        .unwrap_or_else(|| json!({"imported": true})));
        active.insert(&txn).await?;
    }
    txn.commit().await?;
    Ok(summary)
}

fn unique_positive_ids(ids: &[i64], field: &str) -> Result<Vec<i64>, ProjectBundleHistoryError> {
    let mut result = Vec::with_capacity(ids.len());
    let mut seen = HashSet::with_capacity(ids.len());
    for id in ids {
        if *id <= 0 {
            return Err(ProjectBundleHistoryError::Invalid(format!(
                "{field} 必须大于 0"
            )));
        }
        if seen.insert(*id) {
            result.push(*id);
        }
    }
    Ok(result)
}

fn map_required(
    map: &HashMap<i64, i64>,
    old_id: i64,
    field: &str,
) -> Result<i64, ProjectBundleHistoryError> {
    map.get(&old_id).copied().ok_or_else(|| {
        ProjectBundleHistoryError::Invalid(format!("{field} 缺少旧 ID {old_id} 的导入映射"))
    })
}

fn required_bundle_id(value: &str) -> Result<String, ProjectBundleHistoryError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || value.contains(':') {
        return Err(ProjectBundleHistoryError::Invalid(
            "bundle_id 必须是 1-128 个不含冒号的字符".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn remap_workflow_scope(
    scope_table: Option<&str>,
    scope_id: Option<i64>,
    maps: &ProjectBundleEntityIdMaps,
) -> (Option<String>, Option<i64>, bool) {
    let Some(scope_table) = scope_table else {
        return (None, None, scope_id.is_none());
    };
    let Some(scope_id) = scope_id else {
        return (Some(scope_table.to_string()), None, true);
    };
    let mapped = match scope_table {
        "project" | "novel_project" => maps.project_ids.get(&scope_id).copied(),
        "chapter" => maps.chapter_ids.get(&scope_id).copied(),
        "source_snapshot" => maps.source_snapshot_ids.get(&scope_id).copied(),
        _ => None,
    };
    match mapped {
        Some(mapped) => (Some(scope_table.to_string()), Some(mapped), true),
        None => (None, None, false),
    }
}

fn imported_status(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<String>,
) -> (String, bool, Option<String>, bool) {
    if matches!(status, "running" | "interrupted") {
        return (
            "interrupted".to_string(),
            true,
            Some("原机器的运行时任务不能跨机器接管；恢复时将重新提交当前步骤".to_string()),
            status == "running",
        );
    }
    (status.to_string(), can_resume, resume_block_reason, false)
}

fn sanitize_runtime_json(value: Option<Value>) -> Option<Value> {
    let mut value = value?;
    a4n_domain::clear_canvas_transfer_runtime_identity(&mut value);
    Some(value)
}

fn remap_checkpoint_input(
    value: Option<Value>,
    stored_hash: Option<&str>,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<(Option<Value>, Option<String>), ProjectBundleHistoryError> {
    let Some(mut value) = value else {
        if stored_hash.is_some() {
            return Err(ProjectBundleHistoryError::Invalid(
                "调用断点有指纹哈希但缺少原始输入".into(),
            ));
        }
        return Ok((None, None));
    };
    if let Some(stored_hash) = stored_hash {
        let actual = crate::generation::generation_checkpoint_fingerprint_sha256(&value)
            .map_err(|error| ProjectBundleHistoryError::Invalid(error.to_string()))?;
        if actual != stored_hash {
            return Err(ProjectBundleHistoryError::Invalid(
                "调用断点原始输入指纹不一致，不能通过重新计算修复".into(),
            ));
        }
    }
    remap_history_value(&mut value, maps)?;
    // Unfingerprinted history must not become a reusable completed call during import.
    let hash = stored_hash
        .map(|_| crate::generation::generation_checkpoint_fingerprint_sha256(&value))
        .transpose()
        .map_err(|error| ProjectBundleHistoryError::Invalid(error.to_string()))?;
    Ok((Some(value), hash))
}

fn remap_history_json(
    value: Option<Value>,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<Option<Value>, ProjectBundleHistoryError> {
    let Some(mut value) = value else {
        return Ok(None);
    };
    remap_history_value(&mut value, maps)?;
    Ok(Some(value))
}

fn remap_history_value(
    value: &mut Value,
    maps: &ProjectBundleEntityIdMaps,
) -> Result<(), ProjectBundleHistoryError> {
    match value {
        Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if matches!(
                    key.as_str(),
                    "original_input_fingerprint_json" | "original_checkpoint"
                ) {
                    continue;
                }
                if let Some(old_id) = value.as_i64() {
                    let mapping = match key.as_str() {
                        "project_id" => Some((&maps.project_ids, "history.project_id")),
                        "root_project_id" => {
                            Some((&maps.root_project_ids, "history.root_project_id"))
                        }
                        "chapter_id" | "source_chapter_id" => {
                            Some((&maps.chapter_ids, "history.chapter_id"))
                        }
                        "source_snapshot_id" => {
                            Some((&maps.source_snapshot_ids, "history.source_snapshot_id"))
                        }
                        "source_snapshot_chapter_id" => Some((
                            &maps.source_snapshot_chapter_ids,
                            "history.source_snapshot_chapter_id",
                        )),
                        _ => None,
                    };
                    if let Some((map, field)) = mapping {
                        *value = Value::from(map_required(map, old_id, field)?);
                        continue;
                    }
                }
                remap_history_value(value, maps)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                remap_history_value(value, maps)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn remap_generation_ids(
    value: Option<Value>,
    maps: &HashMap<i64, i64>,
) -> Result<Option<Value>, ProjectBundleHistoryError> {
    let Some(mut value) = value else {
        return Ok(None);
    };
    if let Some(items) = value.as_array_mut() {
        for item in items {
            if let Some(old_id) = item.as_i64() {
                *item = Value::from(map_required(
                    maps,
                    old_id,
                    "workflow_checkpoint.generation_run_ids_json",
                )?);
            }
        }
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        ProjectBundleEntityIdMaps, imported_status, remap_checkpoint_input, remap_workflow_scope,
    };

    #[test]
    fn checkpoint_remap_verifies_original_hash_without_inventing_one() {
        let maps = ProjectBundleEntityIdMaps {
            chapter_ids: HashMap::from([(7, 70)]),
            ..Default::default()
        };
        let input = serde_json::json!({"chapter_id": 7, "task": "kept"});
        let hash = crate::generation::generation_checkpoint_fingerprint_sha256(&input).unwrap();
        let (mapped, mapped_hash) =
            remap_checkpoint_input(Some(input.clone()), Some(&hash), &maps).unwrap();
        let expected = serde_json::json!({"chapter_id": 70, "task": "kept"});
        assert_eq!(mapped, Some(expected.clone()));
        assert_eq!(
            mapped_hash,
            Some(crate::generation::generation_checkpoint_fingerprint_sha256(&expected).unwrap())
        );
        let mut corrupt = input.clone();
        corrupt["task"] = serde_json::json!("changed without updating hash");
        assert!(
            remap_checkpoint_input(Some(corrupt), Some(&hash), &maps)
                .unwrap_err()
                .to_string()
                .contains("原始输入指纹不一致")
        );
        assert_eq!(
            remap_checkpoint_input(Some(input), None, &maps).unwrap(),
            (Some(expected), None)
        );
        assert!(remap_checkpoint_input(None, Some(&hash), &maps).is_err());
    }

    #[tokio::test]
    async fn history_project_scope_excludes_unselected_descendants() {
        use super::{selected_project_scope, workflow_run};
        use sea_orm::{
            ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
            QuerySelect, TransactionTrait,
        };
        let db = a4n_migration::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
            CREATE TEMP TABLE workflow_run (id bigint, project_id bigint, root_project_id bigint) ON COMMIT DROP;
            INSERT INTO workflow_run VALUES (1, 10, 10), (2, 20, 10), (3, NULL, 10), (4, 30, 30);
        "#).await.unwrap();
        for (projects, expected) in [(vec![10], vec![1, 3]), (vec![10, 20], vec![1, 2, 3])] {
            let rows = workflow_run::Entity::find()
                .select_only()
                .column(workflow_run::Column::Id)
                .filter(
                    Condition::any()
                        .add(workflow_run::Column::ProjectId.is_in(projects.clone()))
                        .add(workflow_run::Column::RootProjectId.is_in(projects.clone())),
                )
                .filter(selected_project_scope(
                    workflow_run::Column::ProjectId,
                    &projects,
                ))
                .order_by_asc(workflow_run::Column::Id)
                .into_tuple::<i64>()
                .all(&tx)
                .await
                .unwrap();
            assert_eq!(rows, expected);
        }
        tx.rollback().await.unwrap();
    }

    #[test]
    fn running_history_is_imported_as_resumable_interrupted() {
        let (status, can_resume, reason, interrupted) = imported_status("running", false, None);
        assert_eq!(status, "interrupted");
        assert!(can_resume);
        assert!(reason.is_some());
        assert!(interrupted);
    }

    #[test]
    fn workflow_scope_is_remapped_or_blocked_explicitly() {
        let maps = ProjectBundleEntityIdMaps {
            chapter_ids: HashMap::from([(7, 70)]),
            ..Default::default()
        };
        assert_eq!(
            remap_workflow_scope(Some("chapter"), Some(7), &maps),
            (Some("chapter".to_string()), Some(70), true)
        );
        assert_eq!(
            remap_workflow_scope(Some("canvas_graph"), Some(7), &maps),
            (None, None, false)
        );
    }
}
