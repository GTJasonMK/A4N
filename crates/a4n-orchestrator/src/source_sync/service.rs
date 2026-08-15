use super::contracts::{SourceSyncDecisionInput, SourceSyncExecutionContext};
use super::decisions::prepare_decision_execution_context;
use super::preparation::prepared_source_sync_decisions;
use super::rules::source_sync_decisions_require_galgame_preparation;
use crate::OrchestratorContext;
use crate::source_sync::source_sync_db_error;
use a4n_core::error::AppError;
use a4n_core::models::source_sync::source_sync_run;
use a4n_db::source_sync::SourceSyncPlanRecord;

pub struct ApplySourceSyncApplicationInput<'a> {
    pub run: source_sync_run::Model,
    pub decisions: Vec<SourceSyncDecisionInput>,
    pub galgame_prompt_renderer: Option<&'a dyn a4n_prompt::rendering::PromptRenderer>,
}

impl OrchestratorContext {
    pub async fn run_source_sync_apply_validated(
        &self,
        input: ApplySourceSyncApplicationInput<'_>,
    ) -> Result<SourceSyncPlanRecord, AppError> {
        let decisions_json = serde_json::to_value(&input.decisions)?;
        let workflow = self
            .begin_source_sync_apply_workflow_validated(
                input.run.id,
                input.run.derived_project_id,
                input.run.root_project_id,
                input.run.from_snapshot_id,
                input.run.to_snapshot_id,
                &decisions_json,
            )
            .await?;
        let result = self
            .apply_source_sync_decisions_with_context(
                input.run.id,
                input.decisions,
                SourceSyncExecutionContext {
                    galgame: input.galgame_prompt_renderer.map(|prompt_renderer| {
                        super::contracts::GalgameSourceSyncExecutionContext {
                            prompt_renderer,
                            workflow_run_id: workflow.id,
                        }
                    }),
                },
            )
            .await;
        self.finalize_workflow_result(workflow.id, result, "源同步应用")
            .await
    }

    pub(crate) async fn apply_source_sync_decisions_with_context(
        &self,
        sync_run_id: i64,
        decisions: Vec<SourceSyncDecisionInput>,
        execution_context: SourceSyncExecutionContext<'_>,
    ) -> Result<SourceSyncPlanRecord, AppError> {
        if decisions.is_empty() {
            return a4n_db::source_sync::apply_unconflicted_source_sync_records_transaction(
                self.database.connection(),
                sync_run_id,
            )
            .await
            .map_err(source_sync_db_error);
        }
        let prepared_context = if source_sync_decisions_require_galgame_preparation(&decisions) {
            let run = a4n_db::source_sync::load_source_sync_apply_run_record(
                self.database.connection(),
                sync_run_id,
            )
            .await
            .map_err(source_sync_db_error)?;
            prepare_decision_execution_context(self, &run, &decisions, execution_context).await?
        } else {
            Default::default()
        };
        let prepared_decisions = prepared_source_sync_decisions(&decisions, &prepared_context)?;
        a4n_db::source_sync::apply_prepared_source_sync_decisions_records_transaction(
            self.database.connection(),
            sync_run_id,
            prepared_decisions,
        )
        .await
        .map_err(source_sync_db_error)
    }
}
