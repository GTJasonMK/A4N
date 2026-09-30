use super::messages::{
    OptimizeIssue, build_content_optimization_chat_user_message,
    build_content_optimization_patch_user_message, build_content_optimization_user_message,
    is_content_optimization_analysis_prompt, materialize_optimization_issue_evidence,
    optimization_chat_history, parse_optimization_issues,
};
use super::*;
use a4n_db_models::models::ai::optimization_session;
use a4n_domain::project::OptimizationTurnRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::MessageRole;
use a4n_prompt::tera_engine::TeraPromptEngine;
use chrono::Utc;
use serde_json::json;
use std::path::Path;

mod context_read;
mod discard_flow;
mod message_contract;
mod patch_flow;

fn prompts_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../prompts")
}
