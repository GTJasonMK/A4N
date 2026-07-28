use std::collections::HashMap;

use a4n_core::{blueprint_context::blueprint_llm_context, error::AppError};
use a4n_llm::provider::ChatRequest;
use a4n_prompt::blueprint_generation::{
    blueprint_refine_manifest_user_message, build_blueprint_refine_request,
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn preview_blueprint_refine_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        focus: Option<&str>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.preview_blueprint_refine_request(project_id, focus, prompt_renderer, variables)
            .await
    }

    pub async fn preview_blueprint_refine_request(
        &self,
        project_id: i64,
        focus: Option<&str>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        let blueprint = self
            .load_blueprint(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;
        let characters = self.load_blueprint_characters(project_id).await?;
        let relationships = self.load_blueprint_relationships(project_id).await?;
        let blueprint_context = blueprint_llm_context(&blueprint, &characters, &relationships);
        let prompt_content = prompt_renderer
            .render_prompt("blueprint_optimization", variables, Some(project_id))
            .await?;
        let user_message =
            blueprint_refine_manifest_user_message(prompt_renderer, Some(project_id), focus)
                .await?;
        build_blueprint_refine_request(project_id, prompt_content, user_message, &blueprint_context)
    }
}
