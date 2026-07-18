use a4n_core::models::generation::{
    generation_run, generation_run_target, generation_spec, llm_call_trace, prompt_artifact,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const RUN_STATUS_RUNNING: &str = "running";
const RUN_STATUS_COMPLETED: &str = "completed";
const RUN_STATUS_FAILED: &str = "failed";

mod validation;
use validation::*;

mod types;
pub use types::*;
use types::{AttachGenerationRunTargetInput, CreateGenerationRunInput, INTERNAL_PROMPT_TOKENS};

mod runs;
pub use runs::*;

mod refs;
pub use refs::*;

mod prompts;
pub use prompts::*;

#[cfg(test)]
mod tests;
