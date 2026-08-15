mod execution;
mod generation;
mod lifecycle;
mod request;

use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_db::optimize::{OptimizationSessionCreateInput, OptimizationSessionUpdateInput};
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use super::agent_loop::build_first_agent_loop_chat_request;
use super::context::{build_content_optimization_context, optimization_context_manifest};
use super::messages::{build_content_optimization_user_message, optimization_dimensions};
use super::permissions::{
    OptimizeAgentPermissions, insert_optimization_agent_capability_variables,
    optimization_permissions_from_variables,
};
use super::{OrchestratorContext, optimize_db_error, required_available_session_name};
use a4n_prompt::rendering::PromptRenderer;
