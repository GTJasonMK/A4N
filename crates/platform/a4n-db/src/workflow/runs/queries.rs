use super::*;

use sea_orm::FromQueryResult;
use std::collections::HashSet;

use super::super::summaries::GenerationRunSummaryRecord;

use super::projections::{workflow_checkpoint_summary, workflow_run_summary};

#[derive(Debug, FromQueryResult)]
struct WorkflowRunRecoveryQueryRecord {
    id: i64,
    business_key: String,
    status: String,
    error_message: Option<String>,
    updated_at: chrono::NaiveDateTime,
}

pub async fn list_workflow_runs(
    db: &DatabaseConnection,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, WorkflowReadError> {
    let runs = list_workflow_run_models(db, input).await?;
    runs.into_iter().map(workflow_run_summary).collect()
}

pub async fn workflow_run_summary_by_id(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, WorkflowReadError> {
    let run = workflow_run_by_id(db, workflow_run_id).await?;
    workflow_run_summary(run)
}

pub async fn workflow_run_detail(
    db: &DatabaseConnection,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, WorkflowReadError> {
    let workflow_run_id = input.workflow_run_id;
    let run_result = workflow_run_by_id(db, workflow_run_id);
    let checkpoints_result = workflow_checkpoints(db, workflow_run_id);
    let generation_runs_result = async {
        if input.include_generation_runs {
            generation_runs_for_workflow(db, workflow_run_id, 300).await
        } else {
            Ok(Vec::new())
        }
    };
    let (run_result, checkpoints_result, generation_runs_result) =
        futures::join!(run_result, checkpoints_result, generation_runs_result,);
    let run = run_result?;
    let checkpoints = checkpoints_result?;
    let generation_runs = generation_runs_result?;
    Ok(WorkflowRunDetailSummary {
        run: workflow_run_summary(run)?,
        checkpoints: checkpoints
            .into_iter()
            .map(workflow_checkpoint_summary)
            .collect(),
        generation_runs,
    })
}

pub async fn list_workflow_run_models(
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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

pub async fn workflow_run_model_by_id(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowReadError> {
    workflow_run_by_id(db, workflow_run_id).await
}

pub async fn workflow_run_status_record_by_id(
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, WorkflowReadError> {
    let business_key = required_text(business_key.to_string(), "business_key")?;
    latest_workflow_run_by_business_key_model(db, &business_key).await
}

pub async fn latest_workflow_run_status_record_by_business_key(
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_run::Entity::find_by_id(workflow_run_id)
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .ok_or_else(|| WorkflowReadError::NotFound(format!("工作流不存在: {workflow_run_id}")))
}

pub(crate) async fn locked_workflow_run_by_id(
    db: &sea_orm::DatabaseTransaction,
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
    db: &DatabaseConnection,
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

pub(super) async fn locked_latest_workflow_run_by_business_key_model(
    tx: &sea_orm::DatabaseTransaction,
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

pub async fn latest_workflow_run_by_business_key_for_update(
    tx: &sea_orm::DatabaseTransaction,
    business_key: &str,
) -> Result<Option<workflow_run::Model>, WorkflowReadError> {
    let business_key = required_text(business_key.to_string(), "business_key")?;
    locked_latest_workflow_run_by_business_key_model(tx, &business_key).await
}
