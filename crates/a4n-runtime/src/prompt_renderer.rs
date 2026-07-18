use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::Value;

use crate::{AppState, actions::prompt_records::render_prompt_template};

pub(crate) struct RuntimePromptRenderer<'a> {
    state: &'a AppState,
}

impl<'a> RuntimePromptRenderer<'a> {
    pub(crate) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl PromptRenderer for AppState {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        render_prompt_template(self, template_name, variables.clone(), project_id)
            .await
            .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
    }
}

#[async_trait]
impl PromptRenderer for RuntimePromptRenderer<'_> {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.state
            .render_prompt(template_name, variables, project_id)
            .await
    }
}
