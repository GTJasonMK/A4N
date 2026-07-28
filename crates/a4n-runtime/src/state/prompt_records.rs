use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    prompt_override::ProjectPromptOverrideSummary,
    prompt_records::{
        AbRenderInput, AbRenderResult, CategoryInfo, DependencyGraphOutput, PromptEntrySummary,
        PromptExecutionContext, PromptVersionSummary, RemoveOverrideInput, RenderInput,
        SetOverrideInput, UpdatePromptInput, ValidateVarsInput, VersionDiffInput,
    },
};
use a4n_prompt::engine::PromptEntry;
use serde_json::Value;

use super::RuntimeProfile;

pub(crate) struct PromptRecordsRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
    context: PromptExecutionContext,
}

impl<'a> PromptRecordsRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext, profile: RuntimeProfile) -> Self {
        Self {
            orchestrator,
            context: PromptExecutionContext::new(
                profile.uses_sidecars(),
                crate::runtime_config::current_user_id(),
                crate::runtime_config::redacted_current_snapshot_value(),
            ),
        }
    }

    pub(crate) async fn list(&self) -> Result<Vec<PromptEntrySummary>, AppError> {
        self.orchestrator
            .list_prompt_records_validated(&self.context)
            .await
    }

    pub(crate) async fn runtime_entry(&self, name: &str) -> Result<PromptEntry, AppError> {
        self.orchestrator
            .prompt_runtime_entry_validated(&self.context, name)
            .await
    }

    pub(crate) async fn runtime_entries(&self) -> Result<Vec<PromptEntry>, AppError> {
        self.orchestrator
            .prompt_runtime_entries_validated(&self.context)
            .await
    }

    pub(crate) async fn update_many_runtime(
        &self,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.orchestrator
            .update_many_prompt_records_validated(&self.context, updates, author)
            .await
    }

    pub(crate) async fn load_defaults(&self) -> Result<usize, AppError> {
        self.orchestrator
            .load_default_prompt_records_validated(&self.context)
            .await
    }

    pub(crate) async fn update(
        &self,
        input: UpdatePromptInput,
    ) -> Result<PromptEntrySummary, AppError> {
        self.orchestrator
            .update_prompt_record_validated(&self.context, input)
            .await
    }

    pub(crate) async fn reset(&self, name: &str) -> Result<(), AppError> {
        self.orchestrator
            .reset_prompt_record_validated(&self.context, name)
            .await
    }

    pub(crate) async fn version_history(
        &self,
        name: &str,
    ) -> Result<Vec<PromptVersionSummary>, AppError> {
        self.orchestrator
            .prompt_version_history_validated(&self.context, name)
            .await
    }

    pub(crate) async fn render(
        &self,
        user_id: i64,
        input: RenderInput,
    ) -> Result<String, AppError> {
        self.orchestrator
            .render_prompt_record_validated(&self.context, user_id, input)
            .await
    }

    pub(crate) async fn render_template(
        &self,
        template_name: &str,
        variables: HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.orchestrator
            .render_prompt_record_template_validated(
                &self.context,
                template_name,
                variables,
                project_id,
            )
            .await
    }

    pub(crate) async fn render_ab(
        &self,
        user_id: i64,
        input: AbRenderInput,
    ) -> Result<AbRenderResult, AppError> {
        self.orchestrator
            .render_prompt_record_ab_validated(&self.context, user_id, input)
            .await
    }

    pub(crate) async fn set_project_override(
        &self,
        user_id: i64,
        input: SetOverrideInput,
    ) -> Result<(), AppError> {
        self.orchestrator
            .set_prompt_project_override_validated(&self.context, user_id, input)
            .await
    }

    pub(crate) async fn remove_project_override(
        &self,
        user_id: i64,
        input: RemoveOverrideInput,
    ) -> Result<(), AppError> {
        self.orchestrator
            .remove_prompt_project_override_validated(&self.context, user_id, input)
            .await
    }

    pub(crate) async fn list_project_overrides(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, AppError> {
        self.orchestrator
            .list_prompt_project_overrides_validated(&self.context, user_id, project_id)
            .await
    }

    pub(crate) async fn version_diff(&self, input: VersionDiffInput) -> Result<String, AppError> {
        self.orchestrator
            .prompt_version_diff_validated(&self.context, input)
            .await
    }

    pub(crate) async fn validate_variables(
        &self,
        input: ValidateVarsInput,
    ) -> Result<Vec<String>, AppError> {
        self.orchestrator
            .validate_prompt_variables_validated(&self.context, input)
            .await
    }

    pub(crate) async fn dependency_graph(&self) -> Result<DependencyGraphOutput, AppError> {
        self.orchestrator
            .prompt_dependency_graph_validated(&self.context)
            .await
    }

    pub(crate) async fn dependencies(&self, name: &str) -> Result<Vec<String>, AppError> {
        self.orchestrator
            .prompt_dependencies_validated(&self.context, name)
            .await
    }

    pub(crate) async fn reverse_dependencies(&self, name: &str) -> Result<Vec<String>, AppError> {
        self.orchestrator
            .prompt_reverse_dependencies_validated(&self.context, name)
            .await
    }

    pub(crate) async fn categories(&self) -> Result<Vec<CategoryInfo>, AppError> {
        self.orchestrator
            .list_prompt_categories_validated(&self.context)
            .await
    }
}
