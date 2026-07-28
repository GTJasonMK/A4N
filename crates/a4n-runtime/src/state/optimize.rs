//! 内容优化领域 runtime 窄 facade。

use std::collections::HashMap;

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    optimize::{
        StartOptimizeAnalysisGenerationInput, StartOptimizeChatGenerationInput,
        StartOptimizePatchGenerationInput,
    },
};
use a4n_core::error::AppError;
use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_turn,
    optimization_turn_event,
};
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    optimize::{
        ApplyOptimizePatchesApplicationInput, ApplyOptimizePatchesApplicationResult,
        OptimizeAgentPermissions, OptimizeChatGenerationApplicationContext,
        OptimizePatchGenerationApplicationContext, PrepareOptimizeAnalysisApplicationInput,
    },
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

pub(crate) struct OptimizeRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> OptimizeRuntimeFacade<'a> {
    pub(crate) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn optimize_sessions_for_user(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: Option<i64>,
    ) -> Result<Vec<optimization_session::Model>, AppError> {
        self.orchestrator
            .optimize_sessions_for_user(user_id, project_id, chapter_id)
            .await
    }

    pub(crate) async fn optimize_session_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<optimization_session::Model, AppError> {
        self.orchestrator
            .optimize_session_for_user(user_id, session_id)
            .await
    }

    pub(crate) async fn delete_optimize_session_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_optimize_session_for_user_validated(user_id, session_id)
            .await
    }

    pub(crate) async fn record_optimize_agent_event_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        content: &str,
    ) -> Result<(), AppError> {
        self.orchestrator
            .record_optimize_agent_event_for_user_validated(user_id, session_id, content)
            .await
    }

    pub(crate) async fn optimize_turns_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<
        Vec<(
            optimization_turn::Model,
            Vec<optimization_turn_event::Model>,
        )>,
        AppError,
    > {
        self.orchestrator
            .optimize_turns_for_user(user_id, session_id)
            .await
    }

    pub(crate) async fn optimize_patch_sets_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<
        Vec<(
            optimization_patch_set::Model,
            Vec<optimization_patch::Model>,
        )>,
        AppError,
    > {
        self.orchestrator
            .optimize_patch_sets_for_user(user_id, session_id)
            .await
    }

    pub(crate) async fn reject_pending_optimize_patch_sets_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .reject_pending_optimize_patch_sets_for_user_validated(user_id, session_id)
            .await
    }

    pub(crate) async fn apply_optimize_patches_validated(
        &self,
        input: ApplyOptimizePatchesApplicationInput<'_>,
    ) -> Result<ApplyOptimizePatchesApplicationResult, AppError> {
        self.orchestrator
            .apply_optimize_patches_validated(input)
            .await
    }

    pub(crate) async fn create_optimize_analysis_session_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        name: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<optimization_session::Model, AppError> {
        self.orchestrator
            .create_optimize_analysis_session_for_user_validated(
                user_id,
                project_id,
                chapter_number,
                name,
                prompt_renderer,
                variables,
            )
            .await
    }

    pub(crate) async fn prepare_optimize_analysis_session_validated(
        &self,
        input: PrepareOptimizeAnalysisApplicationInput<'_>,
    ) -> Result<optimization_session::Model, AppError> {
        self.orchestrator
            .prepare_optimize_analysis_session_validated(input)
            .await
    }

    pub(crate) async fn create_optimize_chat_session_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        name: &str,
        variables: &HashMap<String, Value>,
    ) -> Result<optimization_session::Model, AppError> {
        self.orchestrator
            .create_optimize_chat_session_for_user_validated(
                user_id,
                project_id,
                chapter_number,
                name,
                variables,
            )
            .await
    }

    pub(crate) async fn start_optimize_chat_generation_task(
        &self,
        user_id: i64,
        input: StartOptimizeChatGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_optimize_chat_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_optimize_chat_generation_task(
        &self,
        context: OptimizeChatGenerationApplicationContext<'_>,
        user_id: i64,
        input: StartOptimizeChatGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_optimize_chat_generation_task(context, user_id, input, handle)
            .await
    }

    pub(crate) async fn start_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: StartOptimizeAnalysisGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_optimize_analysis_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: StartOptimizeAnalysisGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_optimize_analysis_generation_task(user_id, input, handle)
            .await
    }

    pub(crate) async fn start_optimize_patch_generation_task(
        &self,
        user_id: i64,
        input: StartOptimizePatchGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_optimize_patch_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_optimize_patch_generation_task(
        &self,
        context: OptimizePatchGenerationApplicationContext<'_>,
        user_id: i64,
        input: StartOptimizePatchGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_optimize_patch_generation_task(context, user_id, input, handle)
            .await
    }

    pub(crate) async fn build_create_optimize_analysis_preview_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .build_create_optimize_analysis_preview_request_for_user_validated(
                user_id,
                project_id,
                chapter_number,
                prompt_renderer,
                variables,
            )
            .await
    }

    pub(crate) async fn build_prepare_optimize_analysis_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .build_prepare_optimize_analysis_preview_request_for_user_validated(
                user_id,
                session_id,
                prompt_renderer,
                variables,
            )
            .await
    }

    pub(crate) async fn build_send_user_message_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        message: &str,
        permissions_override: Option<&OptimizeAgentPermissions>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .build_send_user_message_preview_request_for_user_validated(
                user_id,
                session_id,
                message,
                permissions_override,
                prompt_renderer,
            )
            .await
    }

    pub(crate) async fn build_propose_patches_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        issue_ids: Option<Vec<String>>,
        prompt_renderer: &dyn PromptRenderer,
        agent_instruction_override: Option<&str>,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .build_propose_patches_preview_request_for_user_validated(
                user_id,
                session_id,
                issue_ids,
                prompt_renderer,
                agent_instruction_override,
            )
            .await
    }
}
