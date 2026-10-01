use a4n_db_models::models::generation::{workflow_checkpoint, workflow_run};
use chrono::NaiveDateTime;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::Value;

use super::types::ImportReverseDbError;

pub async fn latest_completed_reverse_candidate_run(
    db: &DatabaseConnection,
    business_key: &str,
    project_id: i64,
) -> Result<Option<crate::workflow::WorkflowRunSummary>, ImportReverseDbError> {
    let run = workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::Status.eq(a4n_domain::workflow::WORKFLOW_STATUS_COMPLETED))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(db)
        .await?;
    run.map(workflow_run_summary).transpose()
}

pub async fn reverse_candidate_checkpoint_by_step(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    step_key: &str,
) -> Result<Option<workflow_checkpoint::Model>, ImportReverseDbError> {
    workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key))
        .one(db)
        .await
        .map_err(ImportReverseDbError::Database)
}

pub async fn reverse_candidate_workflow_run_by_id_in(
    db: &crate::DbTransaction,
    workflow_run_id: i64,
) -> Result<Option<crate::workflow::WorkflowRunSummary>, ImportReverseDbError> {
    let run = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(db)
        .await?;
    run.map(workflow_run_summary).transpose()
}

pub async fn lock_reverse_candidate_workflow_run_by_id_in(
    db: &crate::DbTransaction,
    workflow_run_id: i64,
) -> Result<Option<crate::workflow::WorkflowRunSummary>, ImportReverseDbError> {
    let mut query = workflow_run::Entity::find_by_id(workflow_run_id);
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query.one(db).await?.map(workflow_run_summary).transpose()
}

fn workflow_run_summary(
    run: workflow_run::Model,
) -> Result<crate::workflow::WorkflowRunSummary, ImportReverseDbError> {
    crate::workflow::workflow_run_summary_from_model(run)
        .map_err(|error| ImportReverseDbError::Internal(error.to_string()))
}

pub async fn lock_reverse_candidate_checkpoint_by_step_in(
    db: &crate::DbTransaction,
    workflow_run_id: i64,
    step_key: &str,
) -> Result<Option<workflow_checkpoint::Model>, ImportReverseDbError> {
    let mut query = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(step_key));
    if db.get_database_backend() != DatabaseBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query.one(db).await.map_err(ImportReverseDbError::Database)
}

pub async fn update_reverse_candidate_checkpoint_artifacts_in(
    db: &crate::DbTransaction,
    checkpoint: workflow_checkpoint::Model,
    artifacts: Value,
    updated_at: NaiveDateTime,
) -> Result<workflow_checkpoint::Model, ImportReverseDbError> {
    let mut active: workflow_checkpoint::ActiveModel = checkpoint.into();
    active.artifacts_json = Set(Some(artifacts));
    active.updated_at = Set(updated_at);
    active
        .update(db)
        .await
        .map_err(ImportReverseDbError::Database)
}
