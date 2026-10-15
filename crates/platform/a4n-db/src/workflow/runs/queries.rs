use super::*;
use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement};
use std::collections::HashSet;

use super::super::summaries::{GenerationRunSummaryRecord, WorkflowCheckpointSummary};

use super::projections::{
    workflow_checkpoint_summary, workflow_run_resume_record, workflow_run_summary,
};
use crate::DbHandle;

#[derive(Debug, FromQueryResult)]
struct WorkflowRunRecoveryQueryRecord {
    id: i64,
    business_key: String,
    status: String,
    error_message: Option<String>,
    updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, FromQueryResult)]
struct WorkflowCheckpointDisplayRecord {
    id: i64,
    workflow_run_id: i64,
    step_key: String,
    step_order: i32,
    status: String,
    artifacts_json: Option<serde_json::Value>,
    error_message: Option<String>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    completed_at: Option<chrono::NaiveDateTime>,
}

pub async fn list_workflow_runs(
    db: &impl crate::DbExecutor,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, WorkflowReadError> {
    let runs = list_workflow_run_models(db, input).await?;
    runs.into_iter().map(workflow_run_summary).collect()
}

pub async fn workflow_run_summary_by_id(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, WorkflowReadError> {
    let run = workflow_run_by_id(db, workflow_run_id).await?;
    workflow_run_summary(run)
}

pub async fn workflow_run_detail(
    db: &impl crate::DbExecutor,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, WorkflowReadError> {
    let workflow_run_id = input.workflow_run_id;
    let include_generation_runs = input.include_generation_runs;
    let include_generation_run_count = input.include_generation_run_count;
    let include_checkpoint_details = input.include_checkpoint_details;
    const GENERATION_RUN_LIMIT: u64 = 300;
    let run_result = workflow_run_by_id(db, workflow_run_id);
    let checkpoints_result =
        workflow_checkpoint_detail_summaries(db, workflow_run_id, include_checkpoint_details);
    let generation_runs_result = async {
        if include_generation_runs {
            generation_runs_for_workflow(db, workflow_run_id, GENERATION_RUN_LIMIT).await
        } else {
            Ok(Vec::new())
        }
    };
    let generation_run_count_result = async {
        if include_generation_run_count && !include_generation_runs {
            generation_run_count_for_workflow(db, workflow_run_id, GENERATION_RUN_LIMIT).await
        } else {
            Ok(0)
        }
    };
    let (run_result, checkpoints_result, generation_runs_result, generation_run_count_result) = futures::join!(
        run_result,
        checkpoints_result,
        generation_runs_result,
        generation_run_count_result,
    );
    let run = run_result?;
    let checkpoints = checkpoints_result?;
    let generation_runs = generation_runs_result?;
    let generation_run_count = if include_generation_runs {
        generation_runs.len() as u64
    } else {
        generation_run_count_result?
    };
    Ok(WorkflowRunDetailSummary {
        run: workflow_run_summary(run)?,
        checkpoints,
        generation_runs,
        generation_run_count,
        checkpoint_details_included: include_checkpoint_details,
    })
}

pub async fn list_workflow_run_models(
    db: &impl crate::DbExecutor,
    input: ListWorkflowRunsInput,
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    let mut query = workflow_run::Entity::find();
    let chapter_ids_filter = input.chapter_ids;
    let has_chapter_ids = chapter_ids_filter.is_some();
    if let Some(project_id) = input.project_id {
        validate_positive_i64(project_id, "project_id")?;
        query = query.filter(workflow_run::Column::ProjectId.eq(project_id));
    }
    if let Some(chapter_id) = input.chapter_id {
        validate_positive_i64(chapter_id, "chapter_id")?;
        query = query.filter(workflow_run::Column::ChapterId.eq(chapter_id));
    }
    if let Some(chapter_ids) = chapter_ids_filter {
        for chapter_id in &chapter_ids {
            validate_positive_i64(*chapter_id, "chapter_id")?;
        }
        if chapter_ids.is_empty() {
            return Ok(Vec::new());
        }
        query = query.filter(workflow_run::Column::ChapterId.is_in(chapter_ids));
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
    query = query
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id);
    if !has_chapter_ids {
        query = query.limit(input.limit.unwrap_or(100).clamp(1, 200));
    }
    query.all(db).await.map_err(WorkflowReadError::Database)
}

pub async fn workflow_runs_by_project_kind(
    db: &impl crate::DbExecutor,
    project_id: i64,
    workflow_kind: &str,
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    workflow_runs_for_project(project_id, workflow_kind)?
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn workflow_run_resume_records_by_project_kind(
    db: &impl crate::DbExecutor,
    project_id: i64,
    workflow_kind: &str,
) -> Result<Vec<WorkflowRunResumeRecord>, WorkflowReadError> {
    Ok(workflow_runs_by_project_kind(db, project_id, workflow_kind)
        .await?
        .into_iter()
        .map(workflow_run_resume_record)
        .collect())
}

fn workflow_runs_for_project(
    project_id: i64,
    workflow_kind: &str,
) -> Result<sea_orm::Select<workflow_run::Entity>, WorkflowReadError> {
    validate_positive_i64(project_id, "project_id")?;
    let workflow_kind = required_text(workflow_kind.to_string(), "workflow_kind")?;
    Ok(workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq(workflow_kind)))
}

/// Same latest-run ordering as the chapter summary projection, bounded to its chapters.
pub async fn latest_workflow_runs_for_chapters(
    db: &impl ConnectionTrait,
    project_id: i64,
    workflow_kind: &str,
    chapter_ids: &[i64],
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    let query = workflow_runs_for_project(project_id, workflow_kind)?;
    for chapter_id in chapter_ids {
        validate_positive_i64(*chapter_id, "chapter_id")?;
    }
    if chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    query
        .filter(workflow_run::Column::ChapterId.is_in(chapter_ids.iter().copied()))
        .distinct_on([workflow_run::Column::ChapterId])
        .order_by_asc(workflow_run::Column::ChapterId)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn latest_workflow_run_summaries_for_chapters(
    db: &DbHandle,
    project_id: i64,
    workflow_kind: &str,
    chapter_ids: &[i64],
) -> Result<Vec<WorkflowRunSummary>, WorkflowReadError> {
    latest_workflow_runs_for_chapters(db.connection(), project_id, workflow_kind, chapter_ids)
        .await?
        .into_iter()
        .map(workflow_run_summary)
        .collect()
}

pub async fn workflow_run_model_by_id(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowReadError> {
    workflow_run_by_id(db, workflow_run_id).await
}

pub async fn workflow_run_summary_by_id_record(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, WorkflowReadError> {
    workflow_run_summary_by_id(db, workflow_run_id).await
}

pub async fn workflow_run_resume_record_by_id(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
) -> Result<WorkflowRunResumeRecord, WorkflowReadError> {
    workflow_run_by_id(db, workflow_run_id)
        .await
        .map(workflow_run_resume_record)
}

pub async fn workflow_run_status_record_by_id(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
) -> Result<WorkflowRunStatusRecord, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_run::Entity::find_by_id(workflow_run_id)
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::Status)
        .column(workflow_run::Column::ErrorMessage)
        .into_model::<WorkflowRunStatusRecord>()
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .ok_or_else(|| WorkflowReadError::NotFound(format!("工作流不存在: {workflow_run_id}")))
}

pub async fn workflow_run_models_by_ids(
    db: &impl crate::DbExecutor,
    workflow_run_ids: &[i64],
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    for workflow_run_id in workflow_run_ids {
        validate_positive_i64(*workflow_run_id, "workflow_run_id")?;
    }
    if workflow_run_ids.is_empty() {
        return Ok(Vec::new());
    }
    workflow_run::Entity::find()
        .filter(workflow_run::Column::Id.is_in(workflow_run_ids.iter().copied()))
        .order_by_asc(workflow_run::Column::Id)
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

/// 按 workflow ID 集合读取 Canvas 终态判断所需的运行 ID 和状态。
pub async fn workflow_run_statuses_by_ids(
    db: &impl crate::DbExecutor,
    workflow_run_ids: &[i64],
) -> Result<Vec<(i64, String)>, WorkflowReadError> {
    for workflow_run_id in workflow_run_ids {
        validate_positive_i64(*workflow_run_id, "workflow_run_id")?;
    }
    if workflow_run_ids.is_empty() {
        return Ok(Vec::new());
    }
    workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::Status)
        .filter(workflow_run::Column::Id.is_in(workflow_run_ids.iter().copied()))
        .order_by_asc(workflow_run::Column::Id)
        .into_tuple::<(i64, String)>()
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn workflow_run_access_record_by_active_task_id(
    db: &impl crate::DbExecutor,
    task_id: &str,
) -> Result<Option<WorkflowRunAccessRecord>, WorkflowReadError> {
    let task_id = required_text(task_id.to_string(), "task_id")?;
    workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::ProjectId)
        .column(workflow_run::Column::RootProjectId)
        .filter(workflow_run::Column::ActiveTaskId.eq(task_id))
        .filter(workflow_run::Column::Status.is_not_in([
            WORKFLOW_STATUS_COMPLETED,
            WORKFLOW_STATUS_FAILED,
            WORKFLOW_STATUS_CANCELLED,
            WORKFLOW_STATUS_SUPERSEDED,
        ]))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .into_model::<WorkflowRunAccessRecord>()
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn latest_workflow_run_by_business_key(
    db: &impl crate::DbExecutor,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, WorkflowReadError> {
    let business_key = required_text(business_key.to_string(), "business_key")?;
    latest_workflow_run_by_business_key_model(db, &business_key).await
}

pub async fn latest_workflow_run_summary_by_business_key(
    db: &impl crate::DbExecutor,
    business_key: &str,
) -> Result<Option<WorkflowRunSummary>, WorkflowReadError> {
    latest_workflow_run_by_business_key(db, business_key)
        .await?
        .map(workflow_run_summary)
        .transpose()
}

pub async fn latest_workflow_run_summary_by_business_key_with_status(
    db: &impl crate::DbExecutor,
    business_key: &str,
    status: &str,
) -> Result<Option<WorkflowRunSummary>, WorkflowReadError> {
    let business_key = required_text(business_key.to_string(), "business_key")?;
    let status = required_text(status.to_string(), "status")?;
    workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .filter(workflow_run::Column::Status.eq(status))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .map(workflow_run_summary)
        .transpose()
}

pub async fn latest_workflow_run_status_record_by_business_key(
    db: &impl crate::DbExecutor,
    business_key: &str,
) -> Result<Option<WorkflowRunStatusRecord>, WorkflowReadError> {
    let business_key = required_text(business_key.to_string(), "business_key")?;
    workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::Status)
        .column(workflow_run::Column::ErrorMessage)
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .into_model::<WorkflowRunStatusRecord>()
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)
}

/// 按 business key 集合读取每个 key 最新的一条工作流记录。
pub async fn latest_workflow_runs_by_business_keys(
    db: &impl crate::DbExecutor,
    business_keys: &[String],
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    let keys = normalized_business_keys(business_keys)?;
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.is_in(keys))
        .distinct_on([workflow_run::Column::BusinessKey])
        .order_by_asc(workflow_run::Column::BusinessKey)
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)?;
    runs.sort_unstable_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(runs)
}

/// 按 business key 集合读取视频恢复所需的每个 key 最新状态。
pub async fn latest_workflow_run_recovery_records_by_business_keys(
    db: &impl crate::DbExecutor,
    business_keys: &[String],
) -> Result<Vec<WorkflowRunRecoveryRecord>, WorkflowReadError> {
    let keys = normalized_business_keys(business_keys)?;
    if keys.is_empty() {
        return Ok(Vec::new());
    }

    let mut runs = workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::BusinessKey)
        .column(workflow_run::Column::Status)
        .column(workflow_run::Column::ErrorMessage)
        .column(workflow_run::Column::UpdatedAt)
        .distinct_on([workflow_run::Column::BusinessKey])
        .order_by_asc(workflow_run::Column::BusinessKey)
        .filter(workflow_run::Column::BusinessKey.is_in(keys))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .into_model::<WorkflowRunRecoveryQueryRecord>()
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)?;
    runs.sort_unstable_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(runs
        .into_iter()
        .map(|run| WorkflowRunRecoveryRecord {
            id: run.id,
            business_key: run.business_key,
            status: run.status,
            error_message: run.error_message,
        })
        .collect())
}

fn normalized_business_keys(business_keys: &[String]) -> Result<Vec<String>, WorkflowReadError> {
    let mut keys = Vec::with_capacity(business_keys.len());
    let mut seen = HashSet::with_capacity(business_keys.len());
    for business_key in business_keys {
        let business_key = required_text(business_key.clone(), "business_key")?;
        if seen.insert(business_key.clone()) {
            keys.push(business_key);
        }
    }
    Ok(keys)
}

pub(in crate::workflow) async fn workflow_run_by_id(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_run::Entity::find_by_id(workflow_run_id)
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .ok_or_else(|| WorkflowReadError::NotFound(format!("工作流不存在: {workflow_run_id}")))
}

pub(crate) async fn locked_workflow_run_by_id<C: ConnectionTrait>(
    db: &C,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_run::Entity::find_by_id(workflow_run_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .ok_or_else(|| WorkflowReadError::NotFound(format!("工作流不存在: {workflow_run_id}")))
}

async fn workflow_checkpoint_detail_summaries<C>(
    db: &C,
    workflow_run_id: i64,
    include_details: bool,
) -> Result<Vec<WorkflowCheckpointSummary>, WorkflowReadError>
where
    C: ConnectionTrait,
{
    if include_details {
        return workflow_checkpoints(db, workflow_run_id)
            .await
            .map(|checkpoints| {
                checkpoints
                    .into_iter()
                    .map(workflow_checkpoint_summary)
                    .collect()
            });
    }
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .order_by_asc(workflow_checkpoint::Column::Id)
        .select_only()
        .column(workflow_checkpoint::Column::Id)
        .column(workflow_checkpoint::Column::WorkflowRunId)
        .column(workflow_checkpoint::Column::StepKey)
        .column(workflow_checkpoint::Column::StepOrder)
        .column(workflow_checkpoint::Column::Status)
        .column(workflow_checkpoint::Column::ArtifactsJson)
        .column(workflow_checkpoint::Column::ErrorMessage)
        .column(workflow_checkpoint::Column::CreatedAt)
        .column(workflow_checkpoint::Column::UpdatedAt)
        .column(workflow_checkpoint::Column::CompletedAt)
        .into_model::<WorkflowCheckpointDisplayRecord>()
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
        .map(|checkpoints| {
            checkpoints
                .into_iter()
                .map(|checkpoint| WorkflowCheckpointSummary {
                    id: checkpoint.id,
                    workflow_run_id: checkpoint.workflow_run_id,
                    step_key: checkpoint.step_key,
                    step_order: checkpoint.step_order,
                    status: checkpoint.status,
                    input_fingerprint_sha256: None,
                    input_fingerprint_json: None,
                    checkpoint_json: None,
                    artifacts_json: checkpoint.artifacts_json,
                    generation_run_ids_json: None,
                    error_message: checkpoint.error_message,
                    created_at: checkpoint.created_at.and_utc().to_rfc3339(),
                    updated_at: checkpoint.updated_at.and_utc().to_rfc3339(),
                    completed_at: checkpoint
                        .completed_at
                        .map(|value| value.and_utc().to_rfc3339()),
                })
                .collect()
        })
}

pub(in crate::workflow) async fn workflow_checkpoints<C>(
    db: &C,
    workflow_run_id: i64,
) -> Result<Vec<workflow_checkpoint::Model>, WorkflowReadError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(workflow_checkpoint::Column::StepOrder)
        .order_by_asc(workflow_checkpoint::Column::Id)
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

async fn generation_runs_for_workflow(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
    limit: u64,
) -> Result<Vec<GenerationRunSummary>, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::WorkflowRunId)
        .column(generation_run::Column::Workflow)
        .column(generation_run::Column::StepKey)
        .column(generation_run::Column::CallKey)
        .column(generation_run::Column::Title)
        .column(generation_run::Column::ModelKind)
        .column(generation_run::Column::Provider)
        .column(generation_run::Column::Model)
        .column(generation_run::Column::Status)
        .column(generation_run::Column::InputFingerprintSha256)
        .column(generation_run::Column::InputFingerprintJson)
        .column(generation_run::Column::RawResponseJson)
        .column(generation_run::Column::ParsedOutputJson)
        .column(generation_run::Column::ValidationJson)
        .column(generation_run::Column::ArtifactRefsJson)
        .column(generation_run::Column::Attempt)
        .column(generation_run::Column::ResponseSummaryJson)
        .column(generation_run::Column::ErrorMessage)
        .column(generation_run::Column::CreatedAt)
        .column(generation_run::Column::CompletedAt)
        .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(generation_run::Column::CreatedAt)
        .limit(limit.clamp(1, 500))
        .into_model::<GenerationRunSummaryRecord>()
        .all(db)
        .await
        .map(|runs| runs.into_iter().map(Into::into).collect())
        .map_err(WorkflowReadError::Database)
}

async fn generation_run_count_for_workflow(
    db: &impl ConnectionTrait,
    workflow_run_id: i64,
    limit: u64,
) -> Result<u64, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let statement = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT COUNT(*)::bigint AS count FROM (SELECT 1 FROM generation_run WHERE workflow_run_id = $1 LIMIT $2) AS limited_runs",
        [workflow_run_id.into(), (limit.clamp(1, 500) as i64).into()],
    );
    let row = db
        .query_one(statement)
        .await
        .map_err(WorkflowReadError::Database)?
        .ok_or_else(|| {
            WorkflowReadError::InvalidData("generation run count query returned no row".into())
        })?;
    let count: i64 = row
        .try_get("", "count")
        .map_err(WorkflowReadError::Database)?;
    u64::try_from(count).map_err(|error| {
        WorkflowReadError::InvalidData(format!("generation run count is negative: {error}"))
    })
}

pub(super) async fn latest_workflow_run_by_business_key_model<C>(
    db: &C,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, WorkflowReadError>
where
    C: ConnectionTrait,
{
    workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub(super) async fn locked_latest_workflow_run_by_business_key_model<C: ConnectionTrait>(
    tx: &C,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, WorkflowReadError> {
    workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn latest_workflow_run_by_business_key_for_update<C: ConnectionTrait>(
    tx: &C,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, WorkflowReadError> {
    let business_key = required_text(business_key.to_string(), "business_key")?;
    locked_latest_workflow_run_by_business_key_model(tx, &business_key).await
}

pub async fn latest_workflow_run_summary_by_business_key_for_update<C: ConnectionTrait>(
    tx: &C,
    business_key: &str,
) -> Result<Option<WorkflowRunSummary>, WorkflowReadError> {
    latest_workflow_run_by_business_key_for_update(tx, business_key)
        .await?
        .map(workflow_run_summary)
        .transpose()
}
