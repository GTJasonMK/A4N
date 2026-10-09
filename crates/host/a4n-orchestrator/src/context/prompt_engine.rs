use std::collections::{HashMap, HashSet};

use a4n_error::AppError;
use a4n_prompt::{
    engine::{
        PromptAssemblyBlock, PromptAssemblyConfig, PromptAssemblySnapshot, PromptCategory,
        PromptEntry, PromptVersion,
    },
    rendering::PromptRenderer,
};
use async_trait::async_trait;

use super::OrchestratorContext;

#[async_trait]
impl PromptRenderer for OrchestratorContext {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let call_key = self.prompt_default_assembly_call_key(template_name)?;
        let snapshot = self
            .resolve_prompt_assembly_snapshot(&call_key, template_name, project_id)
            .await?;
        self.prompt_render_assembled(&snapshot, variables, project_id)
    }

    async fn render_prompt_for_call(
        &self,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let snapshot = self
            .resolve_prompt_assembly_snapshot(call_key, template_name, project_id)
            .await?;
        self.prompt_render_assembled(&snapshot, variables, project_id)
    }
}

impl OrchestratorContext {
    pub(crate) fn prompt_render_template(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.engine.render(template_name, variables, project_id)
    }

    pub(crate) fn prompt_render_assembled(
        &self,
        snapshot: &PromptAssemblySnapshot,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.engine
            .render_assembled(snapshot, variables, project_id)
    }

    pub(crate) fn prompt_template_variables(
        &self,
        prompt_name: &str,
        project_id: Option<i64>,
    ) -> Result<Vec<String>, AppError> {
        self.engine.template_variables(prompt_name, project_id)
    }

    pub(crate) fn prompt_registry_assembly_definitions(
        &self,
        prompt_name: &str,
    ) -> Option<Vec<PromptAssemblyBlock>> {
        self.engine.registry_assembly_definitions(prompt_name)
    }

    pub(crate) fn prompt_registry_assembly_call_keys(&self, prompt_name: &str) -> Vec<String> {
        self.engine.registry_assembly_call_keys(prompt_name)
    }

    pub(crate) fn prompt_default_assembly(&self, prompt_name: &str) -> PromptAssemblyConfig {
        self.engine.default_assembly(prompt_name)
    }

    pub(crate) fn prompt_render_ab(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<(String, String), AppError> {
        self.engine.render_ab(template_name, variables, project_id)
    }

    pub(crate) fn prompt_entry(&self, name: &str) -> Result<PromptEntry, AppError> {
        self.engine.get(name)
    }

    pub(crate) fn prompt_entries(&self) -> Vec<PromptEntry> {
        self.engine.list_all()
    }

    pub(crate) async fn prompt_load_defaults(&self) -> Result<usize, AppError> {
        self.engine.load_defaults().await
    }

    pub(crate) async fn prompt_update_entry(
        &self,
        name: &str,
        content: String,
        author: &str,
    ) -> Result<PromptEntry, AppError> {
        self.engine.update(name, content, author).await
    }

    pub(crate) async fn prompt_update_entries(
        &self,
        updates: Vec<(String, String)>,
        author: &str,
        overwrite: bool,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.engine.update_many(updates, author, overwrite).await
    }

    pub(crate) async fn prompt_reset_entry(&self, name: &str) -> Result<(), AppError> {
        self.engine.reset(name).await
    }

    pub(crate) fn prompt_version_history(
        &self,
        name: &str,
    ) -> Result<Vec<PromptVersion>, AppError> {
        self.engine.version_history(name)
    }

    pub(crate) fn prompt_diff_versions(
        &self,
        name: &str,
        version_a: &str,
        version_b: &str,
    ) -> Result<String, AppError> {
        self.engine.diff(name, version_a, version_b)
    }

    pub(crate) fn prompt_validate_variables(
        &self,
        template_name: &str,
        variables: &HashSet<String>,
    ) -> Result<Vec<String>, AppError> {
        self.engine.validate_variables(template_name, variables)
    }

    pub(crate) fn prompt_set_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(), AppError> {
        self.engine.set_project_override(project_id, name, content)
    }

    pub(crate) fn prompt_remove_project_override(
        &self,
        project_id: i64,
        name: &str,
    ) -> Result<(), AppError> {
        self.engine.remove_project_override(project_id, name)
    }

    pub(crate) fn prompt_categories(&self) -> Result<Vec<PromptCategory>, AppError> {
        self.engine.categories()
    }
}
