//! Runtime-facing contracts for content optimization workers.
//!
//! Session state, patch validation, authorization and generation persistence
//! remain owned by the Optimize application. Runtime only supplies prompt and
//! post-processing adapters and receives stable task results.

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError},
    optimize::{
        AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        CreateOptimizationChatSessionInput, DiscardOptimizationPatchSetInput,
        InspectOptimizationContextInput, OptimizationArticlePatchToolInput,
        OptimizationSessionSummary, PrepareOptimizationAnalysisSessionInput,
        ReadOptimizationContextInput, RecordOptimizeAgentEventInput,
        SearchOptimizationContextInput, SendOptimizationMessageInput,
        StartOptimizeAnalysisGenerationInput, StartOptimizeChatGenerationInput,
        StartOptimizePatchGenerationInput,
    },
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;

#[async_trait]
pub trait OptimizePatchApplyHostPort: Send + Sync {
    async fn postprocess_chapter_after_content_change(
        &self,
        chapter_id: i64,
    ) -> Result<(), AppError>;
}

/// Runtime-facing Agent tools owned by the content-optimization application.
/// The Runtime owns the Agent loop and approvals; Orchestrator owns context,
/// evidence and patch persistence.
#[async_trait]
pub trait OptimizeAgentToolApplicationPort: Send + Sync {
    async fn inspect_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        input: InspectOptimizationContextInput,
    ) -> Result<serde_json::Value, AppError>;

    async fn read_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        input: ReadOptimizationContextInput,
    ) -> Result<serde_json::Value, AppError>;

    async fn search_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        input: SearchOptimizationContextInput,
    ) -> Result<serde_json::Value, AppError>;

    async fn propose_article_patch(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        agent_turn_id: i64,
        input: OptimizationArticlePatchToolInput,
    ) -> Result<serde_json::Value, AppError>;
}

#[async_trait]
pub trait OptimizeSessionMutationApplicationPort: Send + Sync {
    async fn create_analysis_session(
        &self,
        user_id: i64,
        input: AnalyzeChapterInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<OptimizationSessionSummary, AppError>;

    async fn prepare_analysis_session(
        &self,
        user_id: i64,
        input: PrepareOptimizationAnalysisSessionInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<OptimizationSessionSummary, AppError>;

    async fn create_chat_session(
        &self,
        user_id: i64,
        input: CreateOptimizationChatSessionInput,
    ) -> Result<OptimizationSessionSummary, AppError>;

    async fn delete_session(
        &self,
        user_id: i64,
        input: a4n_api_contract::optimize::OptimizeSessionInput,
    ) -> Result<(), AppError>;

    async fn record_agent_event(
        &self,
        user_id: i64,
        input: RecordOptimizeAgentEventInput,
    ) -> Result<(), AppError>;

    async fn discard_pending_patch_sets(
        &self,
        user_id: i64,
        input: a4n_api_contract::optimize::OptimizeSessionInput,
    ) -> Result<(), AppError>;

    async fn discard_patch_set(
        &self,
        user_id: i64,
        input: DiscardOptimizationPatchSetInput,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait OptimizePatchApplicationPort: Send + Sync {
    async fn apply_patches(
        &self,
        user_id: i64,
        input: ApplyOptimizationPatchesInput,
        host_port: &dyn OptimizePatchApplyHostPort,
    ) -> Result<ApplyOptimizationPatchesResponse, AppError>;
}

#[async_trait]
pub trait OptimizePatchPreviewApplicationPort: Send + Sync {
    async fn preview_propose_patches(
        &self,
        user_id: i64,
        input: a4n_api_contract::optimize::ProposeOptimizationPatchesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait OptimizeChatPreviewApplicationPort: Send + Sync {
    async fn preview_send_message(
        &self,
        user_id: i64,
        input: SendOptimizationMessageInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait OptimizeAnalysisPreviewApplicationPort: Send + Sync {
    async fn preview_create_analysis(
        &self,
        user_id: i64,
        input: AnalyzeChapterInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;

    async fn preview_prepare_analysis(
        &self,
        user_id: i64,
        input: PrepareOptimizationAnalysisSessionInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait OptimizeChatGenerationApplicationPort: Send + Sync {
    async fn start_chat_generation(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_chat_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        agent_max_output_tokens: Option<usize>,
        user_id: i64,
        input: StartOptimizeChatGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

#[async_trait]
pub trait OptimizeAnalysisGenerationApplicationPort: Send + Sync {
    async fn start_analysis_generation(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_analysis_generation(
        &self,
        user_id: i64,
        input: StartOptimizeAnalysisGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

#[async_trait]
pub trait OptimizePatchGenerationApplicationPort: Send + Sync {
    async fn start_patch_generation(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_patch_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        user_id: i64,
        input: StartOptimizePatchGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}
