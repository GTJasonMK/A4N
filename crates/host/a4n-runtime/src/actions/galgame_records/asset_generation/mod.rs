use super::GalgameProjectInput;
use crate::AppState;

use super::{GalgameAssetGenerationSummary, GalgameAssetPlan, RunGalgameAssetGenerationInput};

use crate::actions::{image_generation, portrait_generation};
use a4n_api_contract::{
    image::GeneratedImageSummary,
    image_generation::GenerateGalgameImageInput,
    portrait::{GeneratePortraitInput, PortraitSummary},
};
use a4n_error::AppError;
use a4n_galgame_host_contract::GalgameAssetGenerationHostPort;
use a4n_prompt::rendering::PromptRenderer;
use a4n_storage::canonicalize_data_file_path;
use async_trait::async_trait;

pub async fn plan_galgame_assets(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameAssetPlan, String> {
    state
        .galgame_asset_plan_application()
        .plan(user_id, input.project_id)
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
        .galgame_asset_generation_application()
        .run(&host_port, user_id, input)
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
        input: GenerateGalgameImageInput,
    ) -> Result<GeneratedImageSummary, AppError> {
        image_generation::generate_galgame_image(self.state, user_id, input)
            .await
            .map_err(generation_host_error)
    }

    async fn generate_portrait_image(
        &self,
        user_id: i64,
        input: GeneratePortraitInput,
    ) -> Result<PortraitSummary, AppError> {
        portrait_generation::generate_portrait_inline(self.state, user_id, input)
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
