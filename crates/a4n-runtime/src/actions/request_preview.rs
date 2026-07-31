use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

mod chat;
mod contracts;
mod provider;
mod sanitize;
mod status;

pub(crate) use self::chat::{
    chat_request_step_for_state, chat_request_step_with_budget, model_request_preview,
};
pub(crate) use self::contracts::{ChatRequestStepInput, ChatRequestStepWithBudgetInput};
pub use self::contracts::{ModelRequestPreview, ModelRequestPreviewStep};
pub(crate) use self::provider::{blocked_step, image_provider_request_step, provider_request_step};
pub(crate) use self::status::{ChatStatusPreviewStep, chat_status_preview_for_state};

#[cfg(test)]
use self::status::status_step;

#[cfg(test)]
mod tests;
