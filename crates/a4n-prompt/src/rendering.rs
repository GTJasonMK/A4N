use std::collections::HashMap;

use a4n_core::error::AppError;
use async_trait::async_trait;
use serde_json::Value;

use crate::tera_engine::TeraPromptEngine;

#[async_trait]
pub trait PromptRenderer: Send + Sync {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError>;
}

#[async_trait]
impl PromptRenderer for TeraPromptEngine {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.render(template_name, variables, project_id)
    }
}
