//! 源文本同步领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::{project::novel_project, source_sync::source_sync_run};
use a4n_orchestrator::{OrchestratorContext, source_sync::SourceSyncPlan};

pub(crate) struct SourceSyncRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> SourceSyncRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn preview_source_sync_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<SourceSyncPlan, AppError> {
        self.orchestrator
            .preview_source_sync_for_user_validated(user_id, derived_project_id)
            .await
    }

    pub(crate) async fn source_sync_preview_context_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<a4n_orchestrator::project::ProjectFamilyContext, AppError> {
        self.orchestrator
            .source_sync_preview_context_for_user_validated(user_id, derived_project_id)
            .await
    }

    pub(crate) async fn source_sync_run_for_user_validated(
        &self,
        user_id: i64,
        sync_run_id: i64,
    ) -> Result<source_sync_run::Model, AppError> {
        self.orchestrator
            .source_sync_run_for_user_validated(user_id, sync_run_id)
            .await
    }

    pub(crate) async fn run_source_sync_apply_for_user_validated(
        &self,
        user_id: i64,
        run: source_sync_run::Model,
        decisions: Vec<a4n_orchestrator::source_sync::SourceSyncDecisionInput>,
        galgame_prompt_renderer: Option<&dyn a4n_prompt::rendering::PromptRenderer>,
    ) -> Result<source_sync_run::Model, AppError> {
        self.orchestrator
            .run_source_sync_apply_for_user_validated(
                user_id,
                run,
                decisions,
                galgame_prompt_renderer,
            )
            .await
    }

    pub(crate) async fn detach_derived_project_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<novel_project::Model, AppError> {
        self.orchestrator
            .detach_derived_project_for_user_validated(user_id, derived_project_id)
            .await
    }
}
