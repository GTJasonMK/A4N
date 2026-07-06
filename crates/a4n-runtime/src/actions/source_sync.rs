use crate::AppState;
use crate::actions::access::{verify_derived_project_family_context, verify_project_editor};
use a4n_core::error::AppError;
use a4n_core::models::{
    project::novel_project,
    source_sync::{source_sync_conflict, source_sync_run},
};
use a4n_engine::queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_services::project_family::ProjectFamilyContext;
use a4n_services::queue::TaskFamilyContext;
use a4n_services::source_sync::{
    GalgameSourceSyncExecutionContext, SourceSyncExecutionContext, SourceSyncPlan,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub use a4n_services::source_sync::SourceSyncDecisionInput;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplySourceSyncInput {
    pub sync_run_id: i64,
    pub decisions: Vec<SourceSyncDecisionInput>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSyncRunSummary {
    pub id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub mode: String,
    pub status: String,
    pub summary_json: serde_json::Value,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceSyncPlanSummary {
    pub run: SourceSyncRunSummary,
    pub conflicts: Vec<source_sync_conflict::Model>,
}

pub async fn preview_source_sync(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<SourceSyncPlanSummary, AppError> {
    let family =
        verify_derived_project_family_editor_context(state, user_id, derived_project_id).await?;
    let context = TaskFamilyContext::new(
        family.project.id,
        family.root_project.id,
        Some(family.source_snapshot.id),
        "source_sync",
    )?;
    let source_sync = state.source_sync.clone();
    let plan = state
        .task_queue
        .run_tracked_with_family_context(
            QueueTaskType::SourceSync,
            TaskPriority::Medium,
            context,
            || async move { source_sync.preview_source_sync(derived_project_id).await },
        )
        .await?;
    verify_plan_access(state, user_id, &plan).await?;
    Ok(source_sync_plan_summary(plan))
}

pub async fn apply_source_sync_decisions(
    state: &AppState,
    user_id: i64,
    input: ApplySourceSyncInput,
) -> Result<SourceSyncRunSummary, AppError> {
    let run = verify_run_access(state, user_id, input.sync_run_id).await?;
    let context = TaskFamilyContext::new(
        run.derived_project_id,
        run.root_project_id,
        Some(run.from_snapshot_id),
        "source_sync",
    )?;
    let source_sync = state.source_sync.clone();
    let galgame = state.galgame.clone();
    let llm = state.llm.clone();
    let prompt_engine = state.engine.clone();
    let sync_run_id = input.sync_run_id;
    let decisions = input.decisions;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("source_sync_apply:run:{sync_run_id}"),
            workflow_kind: "source_sync_apply".to_string(),
            project_id: Some(run.derived_project_id),
            root_project_id: Some(run.root_project_id),
            source_snapshot_id: Some(run.to_snapshot_id),
            scope_table: Some("source_sync_run".to_string()),
            scope_id: Some(sync_run_id),
            chapter_id: None,
            mode: Some("apply".to_string()),
            input_fingerprint_json: Some(json!({
                "schema_version": 1,
                "workflow": "source_sync_apply",
                "sync_run_id": sync_run_id,
                "derived_project_id": run.derived_project_id,
                "root_project_id": run.root_project_id,
                "from_snapshot_id": run.from_snapshot_id,
                "to_snapshot_id": run.to_snapshot_id,
                "decisions": decisions,
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("apply_decisions".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let workflow_run_id = workflow_run.id;
    let updated = state
        .task_queue
        .run_tracked_with_family_context(
            QueueTaskType::SourceSync,
            TaskPriority::Medium,
            context,
            || async move {
                let execution_context = SourceSyncExecutionContext {
                    galgame: Some(GalgameSourceSyncExecutionContext {
                        service: &galgame,
                        llm: llm.as_ref(),
                        prompt_engine: prompt_engine.as_ref(),
                        workflow_run_id,
                    }),
                };
                source_sync
                    .apply_source_sync_decisions_with_context(
                        sync_run_id,
                        decisions,
                        execution_context,
                    )
                    .await
            },
        )
        .await;
    match updated {
        Ok(updated) => {
            state
                .workflow_run
                .mark_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await?;
            Ok(source_sync_run_summary(&updated))
        }
        Err(err) => {
            let _ = state
                .workflow_run
                .mark_status(
                    workflow_run_id,
                    WORKFLOW_STATUS_FAILED,
                    true,
                    Some(&err.to_string()),
                )
                .await;
            Err(err)
        }
    }
}

pub async fn detach_derived_project(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<novel_project::Model, AppError> {
    verify_derived_project_family_editor_context(state, user_id, derived_project_id).await?;
    state
        .source_sync
        .detach_derived_project(derived_project_id)
        .await
}

async fn verify_derived_project_family_editor_context(
    state: &AppState,
    user_id: i64,
    derived_project_id: i64,
) -> Result<ProjectFamilyContext, AppError> {
    let family = verify_derived_project_family_context(state, user_id, derived_project_id).await?;
    verify_project_editor(state, user_id, family.project.id).await?;
    verify_project_editor(state, user_id, family.root_project.id).await?;
    Ok(family)
}

async fn verify_plan_access(
    state: &AppState,
    user_id: i64,
    plan: &SourceSyncPlan,
) -> Result<(), AppError> {
    verify_project_editor(state, user_id, plan.run.derived_project_id).await?;
    verify_project_editor(state, user_id, plan.run.root_project_id).await
}

async fn verify_run_access(
    state: &AppState,
    user_id: i64,
    sync_run_id: i64,
) -> Result<source_sync_run::Model, AppError> {
    let run = source_sync_run::Entity::find_by_id(sync_run_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("源同步运行不存在: {sync_run_id}")))?;
    verify_project_editor(state, user_id, run.derived_project_id).await?;
    verify_project_editor(state, user_id, run.root_project_id).await?;
    Ok(run)
}

fn source_sync_plan_summary(plan: SourceSyncPlan) -> SourceSyncPlanSummary {
    SourceSyncPlanSummary {
        run: source_sync_run_summary(&plan.run),
        conflicts: plan.conflicts,
    }
}

fn source_sync_run_summary(run: &source_sync_run::Model) -> SourceSyncRunSummary {
    SourceSyncRunSummary {
        id: run.id,
        derived_project_id: run.derived_project_id,
        root_project_id: run.root_project_id,
        from_snapshot_id: run.from_snapshot_id,
        to_snapshot_id: run.to_snapshot_id,
        mode: run.mode.clone(),
        status: run.status.clone(),
        summary_json: run.summary_json.clone(),
        error_message: run.error_message.clone(),
        created_at: run.created_at.to_string(),
        updated_at: run.updated_at.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn source_sync_run_summary_preserves_public_contract_fields() {
        let created_at = NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(2, 3, 4)
            .unwrap();
        let updated_at = NaiveDate::from_ymd_opt(2026, 6, 2)
            .unwrap()
            .and_hms_opt(3, 4, 5)
            .unwrap();
        let run = source_sync_run::Model {
            id: 1,
            derived_project_id: 2,
            root_project_id: 3,
            from_snapshot_id: 4,
            to_snapshot_id: 5,
            mode: "dry_run".to_string(),
            status: "planned".to_string(),
            summary_json: json!({"conflicts": 6}),
            error_message: Some("err".to_string()),
            created_at,
            updated_at,
        };

        let summary = source_sync_run_summary(&run);

        assert_eq!(summary.id, 1);
        assert_eq!(summary.derived_project_id, 2);
        assert_eq!(summary.root_project_id, 3);
        assert_eq!(summary.from_snapshot_id, 4);
        assert_eq!(summary.to_snapshot_id, 5);
        assert_eq!(summary.mode, "dry_run");
        assert_eq!(summary.status, "planned");
        assert_eq!(summary.summary_json, json!({"conflicts": 6}));
        assert_eq!(summary.error_message.as_deref(), Some("err"));
        assert_eq!(summary.created_at, "2026-06-01 02:03:04");
        assert_eq!(summary.updated_at, "2026-06-02 03:04:05");
    }
}
