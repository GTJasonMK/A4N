use super::*;

use super::projections::{
    generation_run_summary, workflow_checkpoint_summary, workflow_run_summary,
};

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
    workflow_run_id: i64,
) -> Result<WorkflowRunDetailSummary, WorkflowReadError> {
    let run = workflow_run_by_id(db, workflow_run_id).await?;
    let checkpoints = workflow_checkpoints(db, workflow_run_id).await?;
    let generation_runs = generation_runs_for_workflow(db, workflow_run_id, 300).await?;
    Ok(WorkflowRunDetailSummary {
        run: workflow_run_summary(run)?,
        checkpoints: checkpoints
            .into_iter()
            .map(workflow_checkpoint_summary)
            .collect(),
        generation_runs: generation_runs
            .into_iter()
            .map(generation_run_summary)
            .collect(),
    })
}

pub async fn list_workflow_run_models(
    db: &DatabaseConnection,
    input: ListWorkflowRunsInput,
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    let mut query = workflow_run::Entity::find();
    if let Some(project_id) = input.project_id {
        validate_positive_i64(project_id, "project_id")?;
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
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)
}

pub async fn workflow_runs_by_project_kind(
    db: &DatabaseConnection,
    project_id: i64,
    workflow_kind: &str,
) -> Result<Vec<workflow_run::Model>, WorkflowReadError> {
    validate_positive_i64(project_id, "project_id")?;
    let workflow_kind = required_text(workflow_kind.to_string(), "workflow_kind")?;
    workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq(workflow_kind))
        .order_by_desc(workflow_run::Column::UpdatedAt)
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

pub async fn workflow_run_model_optional_by_id(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<Option<workflow_run::Model>, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_run::Entity::find_by_id(workflow_run_id)
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

pub(in crate::workflow) async fn workflow_checkpoints(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<Vec<workflow_checkpoint::Model>, WorkflowReadError> {
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
) -> Result<Vec<generation_run::Model>, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    generation_run::Entity::find()
        .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(generation_run::Column::CreatedAt)
        .limit(limit.clamp(1, 500))
        .all(db)
        .await
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
