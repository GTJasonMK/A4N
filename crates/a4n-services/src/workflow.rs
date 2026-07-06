//! 持久化工作流运行与检查点服务。

use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::event::sse::ProgressMeta;
use a4n_core::models::generation::{workflow_checkpoint, workflow_progress_event, workflow_run};
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_PERSISTED_PROGRESS_EVENTS: u64 = 200;

pub const WORKFLOW_STATUS_RUNNING: &str = "running";
pub const WORKFLOW_STATUS_INTERRUPTED: &str = "interrupted";
pub const WORKFLOW_STATUS_FAILED: &str = "failed";
pub const WORKFLOW_STATUS_CANCELLED: &str = "cancelled";
pub const WORKFLOW_STATUS_COMPLETED: &str = "completed";
pub const WORKFLOW_STATUS_SUPERSEDED: &str = "superseded";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertWorkflowRunInput {
    pub business_key: String,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub mode: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub status: String,
    pub current_step_key: Option<String>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertWorkflowCheckpointInput {
    pub workflow_run_id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub generation_run_ids_json: Option<Value>,
    pub error_message: Option<String>,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListWorkflowRunsInput {
    pub project_id: Option<i64>,
    pub workflow_kind: Option<String>,
    pub status: Option<String>,
    pub resumable_only: Option<bool>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WorkflowProgressEventInput<'a> {
    pub workflow_run_id: i64,
    pub task_id: Option<&'a str>,
    pub current: u32,
    pub total: u32,
    pub message: &'a str,
    pub meta: Option<ProgressMeta>,
}

#[derive(Clone)]
pub struct WorkflowRunService {
    db: Arc<DatabaseConnection>,
}

impl WorkflowRunService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn upsert_run(
        &self,
        input: UpsertWorkflowRunInput,
    ) -> Result<workflow_run::Model, AppError> {
        validate_workflow_run_input(&input)?;
        let now = Utc::now().naive_utc();
        let existing = latest_run_by_business_key(self.db.as_ref(), &input.business_key).await?;
        if let Some(existing) = existing.as_ref()
            && workflow_fingerprint_matches(
                existing.input_fingerprint_json.as_ref(),
                input.input_fingerprint_json.as_ref(),
            )
            && should_update_existing_run(existing, &input.status)
        {
            let mut active: workflow_run::ActiveModel = existing.clone().into();
            active.workflow_kind = Set(input.workflow_kind);
            active.project_id = Set(input.project_id);
            active.root_project_id = Set(input.root_project_id);
            active.source_snapshot_id = Set(input.source_snapshot_id);
            active.scope_table = Set(normalize_optional_text(input.scope_table));
            active.scope_id = Set(input.scope_id);
            active.chapter_id = Set(input.chapter_id);
            active.mode = Set(normalize_optional_text(input.mode));
            active.status = Set(input.status);
            active.current_step_key = Set(normalize_optional_text(input.current_step_key));
            active.active_task_id = Set(normalize_optional_text(input.active_task_id));
            active.can_resume = Set(input.can_resume);
            active.resume_block_reason = Set(normalize_optional_text(input.resume_block_reason));
            active.error_message = Set(normalize_optional_text(input.error_message));
            active.updated_at = Set(now);
            active.completed_at = Set(completed_at_for_status(&active.status, now));
            return active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database);
        }

        if let Some(existing) = existing
            && !matches!(
                existing.status.as_str(),
                WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_SUPERSEDED
            )
        {
            let mut active: workflow_run::ActiveModel = existing.into();
            active.status = Set(WORKFLOW_STATUS_SUPERSEDED.to_string());
            active.active_task_id = Set(None);
            active.can_resume = Set(false);
            active.resume_block_reason =
                Set(Some("输入指纹变化，旧工作流已被新运行取代".to_string()));
            active.updated_at = Set(now);
            active.completed_at = Set(Some(now));
            active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
        }

        workflow_run::ActiveModel {
            id: NotSet,
            business_key: Set(input.business_key),
            workflow_kind: Set(input.workflow_kind),
            project_id: Set(input.project_id),
            root_project_id: Set(input.root_project_id),
            source_snapshot_id: Set(input.source_snapshot_id),
            scope_table: Set(normalize_optional_text(input.scope_table)),
            scope_id: Set(input.scope_id),
            chapter_id: Set(input.chapter_id),
            mode: Set(normalize_optional_text(input.mode)),
            input_fingerprint_json: Set(input.input_fingerprint_json),
            status: Set(input.status.clone()),
            current_step_key: Set(normalize_optional_text(input.current_step_key)),
            progress_json: Set(None),
            active_task_id: Set(normalize_optional_text(input.active_task_id)),
            can_resume: Set(input.can_resume),
            resume_block_reason: Set(normalize_optional_text(input.resume_block_reason)),
            error_message: Set(normalize_optional_text(input.error_message)),
            created_at: Set(now),
            updated_at: Set(now),
            completed_at: Set(completed_at_for_status_value(&input.status, now)),
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn mark_status(
        &self,
        workflow_run_id: i64,
        status: &str,
        can_resume: bool,
        error_message: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        validate_workflow_status(status)?;
        let run = self.get_run(workflow_run_id).await?;
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.status = Set(status.to_string());
        active.can_resume = Set(can_resume);
        active.resume_block_reason = Set((!can_resume)
            .then(|| error_message.and_then(non_empty_text))
            .flatten());
        active.error_message = Set(error_message.and_then(non_empty_text));
        active.active_task_id = Set(None);
        active.updated_at = Set(now);
        active.completed_at = Set(completed_at_for_status_value(status, now));
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn mark_active_task(
        &self,
        workflow_run_id: i64,
        task_id: &str,
    ) -> Result<workflow_run::Model, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        let task_id = non_empty_text(task_id)
            .ok_or_else(|| AppError::InvalidArgument("active_task_id 不能为空".to_string()))?;
        let run = self.get_run(workflow_run_id).await?;
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.active_task_id = Set(Some(task_id));
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn mark_current_step(
        &self,
        workflow_run_id: i64,
        current_step_key: &str,
    ) -> Result<workflow_run::Model, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        let current_step_key = required_text(current_step_key.to_string(), "current_step_key")?;
        let run = self.get_run(workflow_run_id).await?;
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.current_step_key = Set(Some(current_step_key));
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn clear_active_task_if_matches(
        &self,
        workflow_run_id: i64,
        task_id: &str,
    ) -> Result<workflow_run::Model, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        let task_id = non_empty_text(task_id)
            .ok_or_else(|| AppError::InvalidArgument("active_task_id 不能为空".to_string()))?;
        let run = self.get_run(workflow_run_id).await?;
        if run.active_task_id.as_deref() != Some(task_id.as_str()) {
            return Ok(run);
        }
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.active_task_id = Set(None);
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn upsert_checkpoint(
        &self,
        input: UpsertWorkflowCheckpointInput,
    ) -> Result<workflow_checkpoint::Model, AppError> {
        validate_positive_id(input.workflow_run_id, "workflow_run_id")?;
        let step_key = required_text(input.step_key, "step_key")?;
        let status = required_text(input.status, "status")?;
        let now = Utc::now().naive_utc();
        let existing = workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(input.workflow_run_id))
            .filter(workflow_checkpoint::Column::StepKey.eq(step_key.as_str()))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        if let Some(existing) = existing {
            let mut active: workflow_checkpoint::ActiveModel = existing.into();
            active.step_order = Set(input.step_order);
            active.status = Set(status);
            active.input_fingerprint_sha256 =
                Set(normalize_optional_text(input.input_fingerprint_sha256));
            active.input_fingerprint_json = Set(input.input_fingerprint_json);
            active.checkpoint_json = Set(input.checkpoint_json);
            active.artifacts_json = Set(input.artifacts_json);
            active.generation_run_ids_json = Set(input.generation_run_ids_json);
            active.error_message = Set(normalize_optional_text(input.error_message));
            active.updated_at = Set(now);
            active.completed_at = Set(input.completed.then_some(now));
            return active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database);
        }

        workflow_checkpoint::ActiveModel {
            id: NotSet,
            workflow_run_id: Set(input.workflow_run_id),
            step_key: Set(step_key),
            step_order: Set(input.step_order),
            status: Set(status),
            input_fingerprint_sha256: Set(normalize_optional_text(input.input_fingerprint_sha256)),
            input_fingerprint_json: Set(input.input_fingerprint_json),
            checkpoint_json: Set(input.checkpoint_json),
            artifacts_json: Set(input.artifacts_json),
            generation_run_ids_json: Set(input.generation_run_ids_json),
            error_message: Set(normalize_optional_text(input.error_message)),
            created_at: Set(now),
            updated_at: Set(now),
            completed_at: Set(input.completed.then_some(now)),
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn record_progress_event(
        &self,
        input: WorkflowProgressEventInput<'_>,
    ) -> Result<workflow_progress_event::Model, AppError> {
        validate_positive_id(input.workflow_run_id, "workflow_run_id")?;
        let total = input.total.max(1);
        let current = input.current.min(total);
        let percent = (current as f32 / total as f32) * 100.0;
        let now = Utc::now().naive_utc();
        let event_index = workflow_progress_event::Entity::find()
            .filter(workflow_progress_event::Column::WorkflowRunId.eq(input.workflow_run_id))
            .order_by_desc(workflow_progress_event::Column::EventIndex)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .map(|event| event.event_index.saturating_add(1))
            .unwrap_or(1);
        let meta_json = input
            .meta
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;

        let progress_json = json!({
            "current": current,
            "total": total,
            "message": input.message,
            "percent": percent,
            "meta": meta_json,
            "updated_at": now.to_string(),
        });
        let mut run: workflow_run::ActiveModel = self.get_run(input.workflow_run_id).await?.into();
        run.progress_json = Set(Some(progress_json));
        run.updated_at = Set(now);
        run.update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        let event = workflow_progress_event::ActiveModel {
            id: NotSet,
            workflow_run_id: Set(input.workflow_run_id),
            task_id: Set(input.task_id.and_then(non_empty_text)),
            event_index: Set(event_index),
            current: Set(current as i32),
            total: Set(total as i32),
            message: Set(input.message.to_string()),
            percent: Set(percent),
            meta_json: Set(meta_json),
            created_at: Set(now),
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;
        self.trim_progress_events(input.workflow_run_id).await?;
        Ok(event)
    }

    pub async fn list_runs(
        &self,
        input: ListWorkflowRunsInput,
    ) -> Result<Vec<workflow_run::Model>, AppError> {
        let mut query = workflow_run::Entity::find();
        if let Some(project_id) = input.project_id {
            validate_positive_id(project_id, "project_id")?;
            query = query.filter(workflow_run::Column::ProjectId.eq(project_id));
        }
        if let Some(workflow_kind) = normalize_optional_text(input.workflow_kind) {
            query = query.filter(workflow_run::Column::WorkflowKind.eq(workflow_kind));
        }
        if let Some(status) = normalize_optional_text(input.status) {
            validate_workflow_status(&status)?;
            query = query.filter(workflow_run::Column::Status.eq(status));
        }
        if input.resumable_only == Some(true) {
            query = query.filter(workflow_run::Column::CanResume.eq(true));
        }
        query
            .order_by_desc(workflow_run::Column::UpdatedAt)
            .limit(input.limit.unwrap_or(100).clamp(1, 200))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn get_run(&self, workflow_run_id: i64) -> Result<workflow_run::Model, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        workflow_run::Entity::find_by_id(workflow_run_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("工作流不存在: {workflow_run_id}")))
    }

    pub async fn latest_run_by_business_key(
        &self,
        business_key: &str,
    ) -> Result<Option<workflow_run::Model>, AppError> {
        let business_key = required_text(business_key.to_string(), "business_key")?;
        latest_run_by_business_key(self.db.as_ref(), &business_key).await
    }

    pub async fn progress_events(
        &self,
        workflow_run_id: i64,
        limit: usize,
    ) -> Result<Vec<workflow_progress_event::Model>, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        workflow_progress_event::Entity::find()
            .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
            .order_by_desc(workflow_progress_event::Column::EventIndex)
            .limit((limit.max(1).min(MAX_PERSISTED_PROGRESS_EVENTS as usize)) as u64)
            .all(self.db.as_ref())
            .await
            .map(|events| events.into_iter().rev().collect())
            .map_err(AppError::Database)
    }

    pub async fn checkpoints(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<workflow_checkpoint::Model>, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
            .order_by_asc(workflow_checkpoint::Column::StepOrder)
            .order_by_asc(workflow_checkpoint::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete_checkpoints_by_keys(
        &self,
        workflow_run_id: i64,
        step_keys: &[&str],
    ) -> Result<u64, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        let step_keys = step_keys
            .iter()
            .map(|key| required_text((*key).to_string(), "step_key"))
            .collect::<Result<Vec<_>, _>>()?;
        if step_keys.is_empty() {
            return Ok(0);
        }
        workflow_checkpoint::Entity::delete_many()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
            .filter(workflow_checkpoint::Column::StepKey.is_in(step_keys))
            .exec(self.db.as_ref())
            .await
            .map(|result| result.rows_affected)
            .map_err(AppError::Database)
    }

    pub async fn completed_checkpoint_by_fingerprint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_sha256: &str,
    ) -> Result<Option<workflow_checkpoint::Model>, AppError> {
        validate_positive_id(workflow_run_id, "workflow_run_id")?;
        let step_key = required_text(step_key.to_string(), "step_key")?;
        let input_fingerprint_sha256 = required_text(
            input_fingerprint_sha256.to_string(),
            "input_fingerprint_sha256",
        )?;
        workflow_checkpoint::Entity::find()
            .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
            .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
            .filter(
                workflow_checkpoint::Column::InputFingerprintSha256.eq(input_fingerprint_sha256),
            )
            .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_COMPLETED))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    async fn trim_progress_events(&self, workflow_run_id: i64) -> Result<(), AppError> {
        let count = workflow_progress_event::Entity::find()
            .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
            .count(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if count <= MAX_PERSISTED_PROGRESS_EVENTS {
            return Ok(());
        }
        let stale_count = count - MAX_PERSISTED_PROGRESS_EVENTS;
        let stale_ids = workflow_progress_event::Entity::find()
            .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
            .order_by_asc(workflow_progress_event::Column::EventIndex)
            .limit(stale_count)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .map(|event| event.id)
            .collect::<Vec<_>>();
        if stale_ids.is_empty() {
            return Ok(());
        }
        workflow_progress_event::Entity::delete_many()
            .filter(workflow_progress_event::Column::Id.is_in(stale_ids))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}

async fn latest_run_by_business_key<C>(
    db: &C,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, AppError>
where
    C: ConnectionTrait,
{
    workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(db)
        .await
        .map_err(AppError::Database)
}

fn validate_workflow_run_input(input: &UpsertWorkflowRunInput) -> Result<(), AppError> {
    required_text(input.business_key.clone(), "business_key")?;
    required_text(input.workflow_kind.clone(), "workflow_kind")?;
    validate_workflow_status(&input.status)?;
    if input.project_id.is_some_and(|id| id <= 0) {
        return Err(AppError::InvalidArgument(
            "project_id 必须大于 0".to_string(),
        ));
    }
    if input.root_project_id.is_some_and(|id| id <= 0) {
        return Err(AppError::InvalidArgument(
            "root_project_id 必须大于 0".to_string(),
        ));
    }
    if input.chapter_id.is_some_and(|id| id <= 0) {
        return Err(AppError::InvalidArgument(
            "chapter_id 必须大于 0".to_string(),
        ));
    }
    Ok(())
}

fn validate_workflow_status(status: &str) -> Result<(), AppError> {
    if matches!(
        status,
        WORKFLOW_STATUS_RUNNING
            | WORKFLOW_STATUS_INTERRUPTED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_CANCELLED
            | WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_SUPERSEDED
    ) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "未知工作流状态: {status}"
        )))
    }
}

fn validate_positive_id(id: i64, label: &str) -> Result<(), AppError> {
    if id > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{label} 必须大于 0")))
    }
}

fn required_text(value: String, label: &str) -> Result<String, AppError> {
    non_empty_text(&value).ok_or_else(|| AppError::InvalidArgument(format!("{label} 不能为空")))
}

fn non_empty_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| non_empty_text(&value))
}

fn workflow_fingerprint_matches(left: Option<&Value>, right: Option<&Value>) -> bool {
    left == right
}

fn should_update_existing_run(existing: &workflow_run::Model, incoming_status: &str) -> bool {
    if incoming_status != WORKFLOW_STATUS_RUNNING {
        return true;
    }
    !matches!(
        existing.status.as_str(),
        WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_SUPERSEDED
    )
}

fn completed_at_for_status(
    status: &ActiveValue<String>,
    now: chrono::NaiveDateTime,
) -> Option<chrono::NaiveDateTime> {
    match status {
        ActiveValue::Set(status) | ActiveValue::Unchanged(status) => {
            completed_at_for_status_value(status, now)
        }
        ActiveValue::NotSet => None,
    }
}

fn completed_at_for_status_value(
    status: &str,
    now: chrono::NaiveDateTime,
) -> Option<chrono::NaiveDateTime> {
    matches!(
        status,
        WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_CANCELLED
            | WORKFLOW_STATUS_SUPERSEDED
    )
    .then_some(now)
}
