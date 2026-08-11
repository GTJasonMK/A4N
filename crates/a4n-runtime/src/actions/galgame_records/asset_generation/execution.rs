use super::{
    AppState, GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameProjectInput,
    RunGalgameAssetGenerationInput,
};
use crate::actions::{image_generation, portrait_generation};
use crate::file_utils::canonicalize_data_file_path;
use a4n_core::error::AppError;
use a4n_orchestrator::{
    galgame::{
        GalgameAssetGenerationHostPort, GalgamePortraitImageGenerationInput,
        GalgameVisualImageGenerationInput,
    },
    media::GeneratedImageSummary,
    portrait::PortraitSummary,
};
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;

pub async fn plan_galgame_assets(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameAssetPlan, String> {
    state
        .orchestrator()
        .plan_galgame_assets_for_user_validated(user_id, input.project_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn run_galgame_asset_generation(
    state: &AppState,
    user_id: i64,
    input: RunGalgameAssetGenerationInput,
) -> Result<GalgameAssetGenerationSummary, String> {
    let host_port = RuntimeGalgameAssetGenerationHostPort { state };
    state
        .orchestrator()
        .run_galgame_asset_generation_for_user_validated(&host_port, user_id, input)
        .await
        .map_err(crate::actions::app_error_message)
}

struct RuntimeGalgameAssetGenerationHostPort<'a> {
    state: &'a AppState,
}

#[async_trait]
impl GalgameAssetGenerationHostPort for RuntimeGalgameAssetGenerationHostPort<'_> {
    async fn render_image_prompt(
        &self,
        template_name: &str,
        variables: &std::collections::HashMap<String, serde_json::Value>,
        project_id: i64,
    ) -> Result<String, AppError> {
        self.state
            .render_prompt(template_name, variables, Some(project_id))
            .await
            .map_err(|error| AppError::InvalidState {
                current: "galgame_prompt_renderer".to_string(),
                operation: error.to_string(),
            })
    }

    async fn generate_visual_image(
        &self,
        user_id: i64,
        input: GalgameVisualImageGenerationInput,
    ) -> Result<GeneratedImageSummary, AppError> {
        image_generation::generate_galgame_image(
            self.state,
            user_id,
            image_generation::GenerateGalgameImageInput {
                project_id: input.project_id,
                prompt: input.prompt,
                image_type: input.image_type,
                config_id: input.config_id,
                chapter_id: input.chapter_id,
                generation_spec_id: input.generation_spec_id,
                target_kind: input.target_kind.to_string(),
                target_entity_kind: input.target_entity_kind.to_string(),
                target_entity_id: input.target_entity_id,
                workflow_run_id: None,
            },
            None,
        )
        .await
        .map_err(generation_host_error)
    }

    async fn generate_portrait_image(
        &self,
        user_id: i64,
        input: GalgamePortraitImageGenerationInput,
    ) -> Result<PortraitSummary, AppError> {
        portrait_generation::generate_portrait_inline(
            self.state,
            user_id,
            portrait_generation::GeneratePortraitInput {
                project_id: input.project_id,
                character_name: input.character_name,
                style: Some(input.style),
                expressions: Some(vec![input.expression]),
                pose: Some(input.pose),
                reference_image_ids: input.reference_image_ids,
            },
        )
        .await
        .map_err(generation_host_error)
    }

    fn validate_data_file(&self, relative_path: &str, label: &str) -> Result<(), AppError> {
        canonicalize_data_file_path(self.state.data_dir(), relative_path, label)
            .map(|_| ())
            .map_err(|error| AppError::InvalidArgument(error.to_string()))
    }
}

fn generation_host_error(operation: String) -> AppError {
    AppError::InvalidState {
        current: "galgame_asset_generation_host".to_string(),
        operation,
    }
}
