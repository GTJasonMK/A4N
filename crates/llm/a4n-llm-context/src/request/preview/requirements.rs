use std::collections::HashMap;

use super::super::super::contracts::{LlmContextManifest, LlmContextPackRequirement};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatMessage;

mod manifest;
mod system;

use manifest::manifest_requirement_inputs;
use system::system_prompt_requirement;

pub(super) type PreviewPackContentKey = (String, String, String);

pub(super) struct PreviewRequirementInputs {
    pub requirements: Vec<LlmContextPackRequirement>,
    pub content_by_key: HashMap<PreviewPackContentKey, String>,
}

pub(super) fn preview_requirement_inputs(
    system_messages: &[ChatMessage],
    manifest: &LlmContextManifest,
) -> Result<PreviewRequirementInputs, AppError> {
    let mut requirements = Vec::new();
    if let Some(system_requirement) = system_prompt_requirement(system_messages)? {
        requirements.push(system_requirement);
    }
    let manifest_inputs = manifest_requirement_inputs(manifest)?;
    requirements.extend(manifest_inputs.requirements);
    Ok(PreviewRequirementInputs {
        requirements,
        content_by_key: manifest_inputs.content_by_key,
    })
}
