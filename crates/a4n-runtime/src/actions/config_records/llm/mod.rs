mod connectivity;
mod contracts;
mod discovery;
mod max_tokens;
mod records;

#[cfg(test)]
mod tests;

pub use connectivity::{preview_test_llm_config_request, test_llm_config};
pub use contracts::{CreateLlmInput, LlmConfigSummary, UpdateLlmInput};
pub use discovery::detect_llm_config_models;
pub use max_tokens::{get_llm_max_tokens_config, update_llm_max_tokens_config};
pub use records::{
    create_llm_config, delete_llm_config, list_llm_config, set_default_llm_config,
    update_llm_config,
};

use contracts::positive_model_output_cap;

#[cfg(test)]
use super::shared::validate_runtime_snapshot_config_id;
#[cfg(test)]
use a4n_core::models::ai::llm_config;
#[cfg(test)]
use connectivity::{
    validate_llm_connection_test_max_tokens, validate_llm_runtime_snapshot_config_id,
};
