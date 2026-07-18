use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use a4n_core::error::AppError;

use crate::engine::{DependencyGraph, PromptEngine, PromptEntry, PromptVersion};

use super::state::TeraPromptEngine;

#[async_trait]
impl PromptEngine for TeraPromptEngine {
    fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.render_prompt(template_name, variables, project_id)
    }

    fn get(&self, name: &str) -> Result<PromptEntry, AppError> {
        self.get_prompt(name)
    }

    fn list_all(&self) -> Vec<PromptEntry> {
        self.all_prompts()
    }

    async fn load_defaults(&self) -> Result<usize, AppError> {
        self.reload_defaults_from_directory().await
    }

    async fn update(
        &self,
        name: &str,
        content: String,
        author: &str,
    ) -> Result<PromptEntry, AppError> {
        self.update_prompt(name, content, author).await
    }

    async fn update_many(
        &self,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.update_prompts(updates, author).await
    }

    async fn reset(&self, name: &str) -> Result<(), AppError> {
        self.reset_prompt(name).await
    }

    fn version_history(&self, name: &str) -> Result<Vec<PromptVersion>, AppError> {
        self.prompt_version_history(name)
    }

    fn get_version(&self, name: &str, version: &str) -> Result<PromptEntry, AppError> {
        self.prompt_at_version(name, version)
    }

    async fn set_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(), AppError> {
        self.set_runtime_project_override(project_id, name, content)
            .await
    }

    async fn remove_project_override(&self, project_id: i64, name: &str) -> Result<(), AppError> {
        self.remove_runtime_project_override(project_id, name).await
    }

    fn diff(&self, name: &str, version_a: &str, version_b: &str) -> Result<String, AppError> {
        self.prompt_diff(name, version_a, version_b)
    }

    fn validate_variables(
        &self,
        template_name: &str,
        variables: &HashSet<String>,
    ) -> Result<Vec<String>, AppError> {
        self.validate_prompt_variables(template_name, variables)
    }

    fn dependency_graph(&self) -> DependencyGraph {
        self.prompt_dependency_graph()
    }

    fn render_ab(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<(String, String), AppError> {
        self.render_prompt_ab(template_name, variables, project_id)
    }

    fn registry(&self) -> Arc<RwLock<crate::registry::PromptRegistry>> {
        self.prompt_registry()
    }
}
