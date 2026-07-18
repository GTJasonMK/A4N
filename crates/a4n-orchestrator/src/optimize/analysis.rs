mod execution;
mod lifecycle;
mod request;

use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_db::optimize::{OptimizationSessionCreateInput, OptimizationSessionUpdateInput};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use super::agent_loop::{build_first_agent_loop_chat_request, insert_current_chapter_content};
use super::agent_loop_executor::ExplicitAnalysisAgentLoopInput;
use super::context::{build_content_optimization_context, optimization_context_manifest};
use super::messages::{
    build_content_optimization_user_message, is_content_optimization_analysis_prompt,
    optimization_dimensions,
};
use super::permissions::{
    OptimizeAgentPermissions, insert_optimization_agent_capability_variables,
};
use super::{
    OptimizeLlmStream, OptimizeProgress, OrchestratorContext, optimize_db_error,
    report_optimize_progress, required_available_session_name,
};
use a4n_prompt::rendering::PromptRenderer;
