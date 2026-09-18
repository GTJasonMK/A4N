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
}

#[derive(Debug, Clone, Default)]
pub struct ProjectBundleHistoryImportSummary {
    pub conversations_imported: usize,
    pub messages_imported: usize,
    pub workflow_runs_imported: usize,
    pub workflow_runs_marked_interrupted: usize,
    pub generation_runs_imported: usize,
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
        .order_by_asc(llm_call_trace::Column::Id)
        .all(db)
        .await?;

    let usage_condition = Condition::any()
        .add(model_usage_record::Column::ProjectId.is_in(project_ids.clone()))
        .add(model_usage_record::Column::RootProjectId.is_in(project_ids));
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
) -> Result<ProjectBundleHistoryImportSummary, ProjectBundleHistoryError> {
    let bundle_id = required_bundle_id(bundle_id)?;
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
            .and_then(|id| maps.chapter_ids.get(&id).copied());
        let source_snapshot_id = source
            .source_snapshot_id
            .and_then(|id| maps.source_snapshot_ids.get(&id).copied());
        let (status, can_resume, resume_block_reason, interrupted) = imported_status(
            &source.status,
            source.can_resume,
            source.resume_block_reason.clone(),
        );
        let mut active: workflow_run::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.business_key = Set(format!("{bundle_id}:{}", source.business_key));
        active.project_id = Set(project_id);
        active.root_project_id = Set(root_project_id);
        active.source_snapshot_id = Set(source_snapshot_id);
        active.scope_id = Set(None);
        active.chapter_id = Set(chapter_id);
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
        active.generation_run_ids_json = Set(remap_generation_ids(
            source.generation_run_ids_json.clone(),
            &generation_ids,
        ));
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
        active.insert(&txn).await?;
    }
    for source in &rows.llm_call_traces {
        let mut active: llm_call_trace::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(source
            .project_id
            .and_then(|id| maps.project_ids.get(&id).copied()));
        active.root_project_id = Set(source
            .root_project_id
            .and_then(|id| maps.root_project_ids.get(&id).copied()));
        active.workflow_run_id = Set(source
            .workflow_run_id
            .and_then(|id| workflow_ids.get(&id).copied()));
        active.generation_run_id = Set(source
            .generation_run_id
            .and_then(|id| generation_ids.get(&id).copied()));
        active.context_plan_json = Set(sanitize_runtime_json(source.context_plan_json.clone()));
        active.context_pack_refs_json =
            Set(sanitize_runtime_json(source.context_pack_refs_json.clone()));
        active.insert(&txn).await?;
    }
    for source in &rows.model_usage_records {
        let mut active: model_usage_record::ActiveModel = source.clone().into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(source
            .project_id
            .and_then(|id| maps.project_ids.get(&id).copied()));
        active.root_project_id = Set(source
            .root_project_id
            .and_then(|id| maps.root_project_ids.get(&id).copied()));
        active.workflow_run_id = Set(source
            .workflow_run_id
            .and_then(|id| workflow_ids.get(&id).copied()));
        active.generation_run_id = Set(source
            .generation_run_id
            .and_then(|id| generation_ids.get(&id).copied()));
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

fn remap_generation_ids(value: Option<Value>, maps: &HashMap<i64, i64>) -> Option<Value> {
    let mut value = value?;
    if let Some(items) = value.as_array_mut() {
        for item in items {
            if let Some(old_id) = item.as_i64() {
                if let Some(new_id) = maps.get(&old_id) {
                    *item = Value::from(*new_id);
                }
            }
        }
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::imported_status;

    #[test]
    fn running_history_is_imported_as_resumable_interrupted() {
        let (status, can_resume, reason, interrupted) = imported_status("running", false, None);
        assert_eq!(status, "interrupted");
        assert!(can_resume);
        assert!(reason.is_some());
        assert!(interrupted);
    }
}
