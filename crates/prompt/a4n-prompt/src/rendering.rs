use std::collections::HashMap;

use a4n_error::AppError;
use async_trait::async_trait;
use serde_json::Value;

use crate::tera_engine::TeraPromptEngine;

pub use a4n_prompt_contract::rendering::PromptRenderer;

#[async_trait]
impl PromptRenderer for TeraPromptEngine {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let call_key = format!("prompt:{template_name}");
        PromptRenderer::render_prompt_for_call(
            self,
            &call_key,
            template_name,
            variables,
            project_id,
        )
        .await
    }

    async fn render_prompt_for_call(
        &self,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let snapshot = crate::engine::PromptAssemblySnapshot {
            call_key: call_key.to_string(),
            prompt_name: template_name.to_string(),
            config: self.default_assembly(template_name),
            source: crate::engine::PromptAssemblySource::SystemDefault,
        };
        self.render_assembled(&snapshot, variables, project_id)
    }
}
