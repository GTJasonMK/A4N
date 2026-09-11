use std::collections::HashMap;

use a4n_db::blueprint_context::blueprint_llm_context;
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::blueprint_generation::build_blueprint_refine_request;
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
        let (blueprint, characters, relationships) = futures::join!(
            self.load_blueprint(project_id),
            self.load_blueprint_characters(project_id),
            self.load_blueprint_relationships(project_id),
        );
        let blueprint = blueprint?.ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;
        let characters = characters?;
        let relationships = relationships?;
        let blueprint_context = blueprint_llm_context(&blueprint, &characters, &relationships);
        build_blueprint_refine_request(
            prompt_renderer,
            project_id,
            focus,
            variables,
            &blueprint_context,
        )
        .await
    }
}
