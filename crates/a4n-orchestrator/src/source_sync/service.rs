use super::contracts::{SourceSyncDecisionInput, SourceSyncExecutionContext, SourceSyncPlan};
use super::decisions::prepare_decision_execution_context;
use super::preparation::prepared_source_sync_decisions;
use super::rules::source_sync_decisions_require_galgame_preparation;
use crate::OrchestratorContext;
use crate::source_sync::source_sync_db_error;
use a4n_core::error::AppError;
use a4n_core::models::source_sync::source_sync_run;
impl OrchestratorContext {
    pub async fn preview_source_sync(
        &self,
        derived_project_id: i64,
    ) -> Result<SourceSyncPlan, AppError> {
        let preview = a4n_db::source_sync::create_source_sync_preview_records(
            self.db.connection(),
            derived_project_id,
        )
        .await
        .map_err(source_sync_db_error)?;
        Ok(SourceSyncPlan {
            run: preview.run,
            conflicts: preview.conflicts,
        })
    }

    pub async fn apply_source_sync_decisions(
        &self,
        sync_run_id: i64,
        decisions: Vec<SourceSyncDecisionInput>,
    ) -> Result<source_sync_run::Model, AppError> {
        self.apply_source_sync_decisions_with_context(
            sync_run_id,
            decisions,
            SourceSyncExecutionContext::default(),
        )
        .await
    }

    pub async fn apply_source_sync_decisions_with_context(
        &self,
        sync_run_id: i64,
        decisions: Vec<SourceSyncDecisionInput>,
        execution_context: SourceSyncExecutionContext<'_>,
    ) -> Result<source_sync_run::Model, AppError> {
        if decisions.is_empty() {
            return a4n_db::source_sync::apply_unconflicted_source_sync_records_transaction(
                self.db.connection(),
                sync_run_id,
            )
            .await
            .map_err(source_sync_db_error);
        }
        let prepared_context = if source_sync_decisions_require_galgame_preparation(&decisions)? {
            let run = a4n_db::source_sync::load_source_sync_apply_run_record(
                self.db.connection(),
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
            self.db.connection(),
            sync_run_id,
            prepared_decisions,
        )
        .await
        .map_err(source_sync_db_error)
    }
}
