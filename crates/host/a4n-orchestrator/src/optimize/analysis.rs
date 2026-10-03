mod execution;
mod generation;
mod lifecycle;
mod request;

use a4n_api_contract::optimize::OptimizationAgentPermissions;
use a4n_db::optimize::OptimizationSessionCreateInput;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

use super::agent_loop::build_first_agent_loop_chat_request;
use super::context::{build_content_optimization_context, optimization_context_manifest};
use super::messages::{build_content_optimization_user_message, optimization_dimensions};
use super::permissions::{
    insert_optimization_agent_capability_variables, optimization_permissions_from_variables,
};
use super::{OrchestratorContext, optimize_db_error, required_available_session_name};
use a4n_prompt::rendering::PromptRenderer;
