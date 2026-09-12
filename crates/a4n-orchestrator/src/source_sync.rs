//! 源文本同步计划、冲突决策与 Galgame 场景再生成编排。

mod contracts;
mod decisions;
mod preparation;
mod projection;
mod rules;
mod service;
mod workflow_rules;

pub use contracts::{
    GalgameSourceSyncExecutionContext, SourceSyncDecisionInput, SourceSyncExecutionContext,
};
pub(in crate::source_sync) use projection::source_sync_plan_summary;
pub(in crate::source_sync) use service::ApplySourceSyncApplicationInput;
pub(crate) use workflow_rules::source_sync_apply_workflow_run_input;

use crate::prompt_records::PromptRendererAdapter;
use a4n_api_contract::source_sync::{
    ApplySourceSyncInput, SourceSyncPlanSummary, SourceSyncProjectInput,
};
use a4n_error::AppError;
use a4n_queue::TaskFamilyContext;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};

#[async_trait::async_trait]
impl a4n_api_contract::source_sync::SourceSyncReadApplicationPort for crate::OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        input: SourceSyncProjectInput,
    ) -> Result<SourceSyncPlanSummary, AppError> {
        let family = self
            .source_sync_preview_context_for_user_validated(user_id, input.derived_project_id)
            .await?;
        let context = TaskFamilyContext::new(
            family.project.id,
            family.root_project.id,
            Some(family.source_snapshot.id),
            "source_sync",
        )?;
        let plan = self
            .queue_run_tracked_with_family_context(
                QueueTaskKind::SourceSync,
                TaskPriority::Medium,
                context,
                || async move { self.preview_source_sync(input.derived_project_id).await },
            )
            .await?;
        source_sync_plan_summary(plan)
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::source_sync::SourceSyncMutationApplicationPort
    for crate::OrchestratorContext
{
    async fn apply(
        &self,
        user_id: i64,
        input: ApplySourceSyncInput,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<SourceSyncPlanSummary, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        let run = self
            .source_sync_run_for_user_validated(user_id, input.sync_run_id)
            .await?;
        let context = TaskFamilyContext::new(
            run.derived_project_id,
            run.root_project_id,
            Some(run.from_snapshot_id),
            "source_sync",
        )?;
        let plan = self
            .queue_run_tracked_with_family_context(
                QueueTaskKind::SourceSync,
                TaskPriority::Medium,
                context,
                || async move {
                    self.run_source_sync_apply_validated(ApplySourceSyncApplicationInput {
                        run,
                        decisions: input.decisions,
                        galgame_prompt_renderer: Some(&prompt_renderer),
                    })
                    .await
                },
            )
            .await?;
        source_sync_plan_summary(plan)
    }

    async fn detach(&self, user_id: i64, input: SourceSyncProjectInput) -> Result<(), AppError> {
        self.detach_derived_project_for_user_validated(user_id, input.derived_project_id)
            .await
    }
}

#[cfg(test)]
use contracts::{RefreshSpecPayload, RegenerateGalgameScenePayload};
#[cfg(test)]
use preparation::prepared_source_sync_decisions;
#[cfg(test)]
use rules::source_sync_decisions_require_galgame_preparation;

#[cfg(test)]
mod tests;

pub(crate) fn source_sync_db_error(
    error: a4n_db::source_sync::SourceSyncDbError,
) -> a4n_error::AppError {
    match error {
        a4n_db::source_sync::SourceSyncDbError::InvalidArgument(message) => {
            a4n_error::AppError::InvalidArgument(message)
        }
        a4n_db::source_sync::SourceSyncDbError::InvalidState { current, operation } => {
            a4n_error::AppError::InvalidState { current, operation }
        }
        a4n_db::source_sync::SourceSyncDbError::NotFound(message) => {
            a4n_error::AppError::NotFound(message)
        }
        a4n_db::source_sync::SourceSyncDbError::Forbidden(message) => {
            a4n_error::AppError::Forbidden(message)
        }
        a4n_db::source_sync::SourceSyncDbError::Internal(message) => {
            a4n_error::AppError::Internal(anyhow::anyhow!(message))
        }
        a4n_db::source_sync::SourceSyncDbError::Database(error) => {
            a4n_error::AppError::Database(error)
        }
    }
}
