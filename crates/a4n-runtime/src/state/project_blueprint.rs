use std::collections::HashMap;

use a4n_api_contract::blueprint::{StartBlueprintGenerationInput, StartBlueprintRefinementInput};
use a4n_core::{
    error::AppError,
    models::project::{blueprint_version, novel_blueprint},
};
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::GenerationTaskAccessError;
use a4n_orchestrator::{OrchestratorContext, project::BlueprintRestoreResult};
use a4n_prompt::blueprint_generation::BlueprintGenerationPreviewStep;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(crate) struct ProjectBlueprintRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectBlueprintRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn novel_blueprint_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Option<novel_blueprint::Model>, AppError> {
        self.orchestrator
            .novel_blueprint_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn blueprint_versions_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<blueprint_version::Model>, AppError> {
        self.orchestrator
            .blueprint_versions_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn start_blueprint_generation_task(
        &self,
        user_id: i64,
        input: StartBlueprintGenerationInput,
    ) -> Result<a4n_orchestrator::GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_blueprint_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_blueprint_refinement_task(
        &self,
        user_id: i64,
        input: StartBlueprintRefinementInput,
    ) -> Result<a4n_orchestrator::GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_blueprint_refinement_task(user_id, input)
            .await
    }

    pub(crate) async fn run_blueprint_generation_task(
        &self,
        context: a4n_orchestrator::blueprint::BlueprintGenerationApplicationContext<'_>,
        user_id: i64,
        input: StartBlueprintGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_blueprint_generation_task(context, user_id, input, handle)
            .await
    }

    pub(crate) async fn run_blueprint_refinement_task(
        &self,
        context: a4n_orchestrator::blueprint::BlueprintRefinementApplicationContext<'_>,
        user_id: i64,
        input: StartBlueprintRefinementInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_blueprint_refinement_task(context, user_id, input, handle)
            .await
    }

    pub(crate) async fn diff_blueprint_versions_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        version_a: &str,
        version_b: &str,
    ) -> Result<Value, AppError> {
        self.orchestrator
            .diff_blueprint_versions_for_user_validated(user_id, project_id, version_a, version_b)
            .await
    }

    pub(crate) async fn restore_blueprint_version_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        version: &str,
    ) -> Result<BlueprintRestoreResult, AppError> {
        self.orchestrator
            .restore_blueprint_version_for_user_validated(user_id, project_id, version)
            .await
    }

    pub(crate) async fn update_blueprint_world_setting_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        world_setting: Value,
    ) -> Result<Value, AppError> {
        self.orchestrator
            .update_blueprint_world_setting_for_user_validated(user_id, project_id, world_setting)
            .await
    }

    pub(crate) async fn preview_blueprint_generation_requests_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        novel_title: &str,
        genre: &str,
        target_length: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<Vec<BlueprintGenerationPreviewStep>, AppError> {
        self.orchestrator
            .preview_blueprint_generation_requests_for_user_validated(
                user_id,
                project_id,
                novel_title,
                genre,
                target_length,
                prompt_renderer,
                variables,
            )
            .await
    }

    pub(crate) async fn preview_blueprint_refine_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        focus: Option<&str>,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .preview_blueprint_refine_request_for_user_validated(
                user_id,
                project_id,
                focus,
                prompt_renderer,
                variables,
            )
            .await
    }
}
